#![feature(str_as_str)]

mod cached_download;

use crate::cached_download::cached_download;
use proc_macro::TokenStream;
use quote::quote;
use regex::Regex;
use std::fs;
use std::path::PathBuf;
use syn::LitStr;

fn root_dir() -> PathBuf {
    // this should resolve to the root directory of the crate CALLING the macro (ie, where dioxus actually is)
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(manifest_dir)
}

fn cache_dir() -> PathBuf {
    // appropriate place to save files
    // target is used by rustc for caching anyways, dioxus is fine with it, lol
    let cache_dir = root_dir().join("target").join("google-fonts-cache");
    fs::create_dir_all(&cache_dir).unwrap();
    cache_dir
}

fn expand(url: &str) -> proc_macro2::TokenStream {
    let cache_dir = cache_dir();

    // download google fonts css, or serve from filesystem if cached
    // the ONLY caching is done at the https fetch step, so there is some stuff that is recomputed
    // on every build, but not significant enough to really be an issue, and makes it
    // conceptually much easier
    let csspath = cached_download(url, &cache_dir);

    // since it might be on the filesystem (if cahced) ANYWAYS, read from fs. if we fetched it just
    // saves to fs and then we re-read it. not a huge deal
    let css = fs::read_to_string(&csspath).unwrap();

    // css url(): in this case, font files!
    // claude wrote this regex cause who cares
    let re = Regex::new(r#"url\(['"]?(https?://[^)'"]+)['"]?\)"#).unwrap();

    // prep to be used as a format string, escape bracket literals
    let css = css.replace("{", "{{").replace("}", "}}");

    // for every url (which is a font we must download), save to a list, and prep to be re-inserted via format!
    // im doing 2 things at once here, both extracting all the URLs to a vec, and also replacing
    // them with format shit. yes its a little weird im inserting into a vec from inside a
    // find/replace closure, but eh it works
    let mut urls = Vec::<String>::new();
    let css = re.replace_all(&*css, |caps: &regex::Captures| {
        let original_url = &caps[1];
        urls.push(original_url.to_string());
        //
        "url({})"
    });

    // i dont want to shit out the entire css file into the source tree (this is a MACRO after all!)
    // so save to file, and include_str!. yes it kinda adds another trip to the filesystem, but also
    // i think that's better than putting it into the source! also i worry about inserting random css
    // chars as an unsafe rust string, god knows what would happen
    let csshash = format!("{:x}.patch.css", md5::compute(&*url));
    let css_format_file = cache_dir.join(&csshash);
    // if we write to the file despite it not changing, the dioxus cli freaks out and hot reloads.
    // so, if this exists, its fucking deterministic and we dont need to overwrite to fs who cares
    if !css_format_file.exists() {
        fs::write(&css_format_file, css.as_str()).unwrap();
    }

    let root_dir = root_dir();
    // for every font URL in the CSS
    let paths = urls
        .iter()
        .map(|url| {
            // if not on disk, download. and return the path where it is
            let abs_path = cached_download(url, &cache_dir);
            // dioxus asset! macro is RELATIVE TO CRATE ROOT. so format that way
            // something something filesystem independent who cares
            let rel_path = abs_path.strip_prefix(&root_dir).unwrap();
            format!("/{}", rel_path.display())
        })
        .collect::<Vec<String>>();

    // pathbuf to string
    let cfstring = css_format_file.display().to_string();

    // essentially:
    // format!(include_str!(css_format_file), asset!(path1), asset!(path2), ...)
    // offloads "compute asset path and insert into css" to the dioxus/parent crate compile step
    // cause its a macro and we cant "compute" asset! inside this crate realistically
    quote! {
        rsx! {
            style {
                { format!(include_str!(#cfstring), #(asset!(#paths)),*) }
            }
        }
    }
}

#[proc_macro]
pub fn embed_google_font(input: TokenStream) -> TokenStream {
    let url = syn::parse_macro_input!(input as LitStr).value();
    expand(&url).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_output() {
        let out = expand(
            "https://fonts.googleapis.com/css2?family=Atkinson+Hyperlegible+Mono:ital,wght@0,200..800;1,200..800&family=Atkinson+Hyperlegible+Next:ital,wght@0,200..800;1,200..800&display=swap",
        );
        println!("{}", out);
    }
}
