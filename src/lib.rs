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

fn expand_google_font(url: &str) -> proc_macro2::TokenStream {
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

/// downloads the necessary CSS and font files from google fonts, caches them, and embeds them with
/// the [dioxus `asset!()` macro](https://dioxuslabs.com/learn/0.7/essentials/ui/assets/) so they
/// can be self-hosted (web) or used offline (desktop/mobile).
///
/// # Arguments
///
/// * `input`: a string of a URL calling the [Google Fonts CSS API 2](https://developers.google.com/fonts/docs/css2),
/// (which is easily obtainable via the [google fonts website](https://fonts.google.com/)). See
/// crate README for documentation for obtaining this URL
///
/// # Examples
///
/// ```
/// use dioxus::prelude::*;
/// use dioxus_google_font_embedder::embed_google_font;
///
/// fn main() {
///     dioxus::launch(App);
/// }
///
/// #[component]
/// fn App() -> Element {
///     rsx! {
///         {embed_google_font!("https://fonts.googleapis.com/css2?family=Atkinson+Hyperlegible+Next:ital,wght@0,200..800;1,200..800&display=swap")}
///         p {
///             font_family: "Atkinson Hyperlegible Next",
///             "This text will be rendered using a Google Font that is downloaded, cached, and embedded at compile time!"
///         }
///     }
/// }
/// ```
#[proc_macro]
pub fn embed_google_font(input: TokenStream) -> TokenStream {
    let url = syn::parse_macro_input!(input as LitStr).value();
    expand_google_font(&url).into()
}


fn expand_asset_url(url: &str) -> proc_macro2::TokenStream {
    let cache_dir = cache_dir();
    // download file and cache at compile time, return path
    let cached_asset_path = cached_download(&*url, &cache_dir);

    // format path relative to calling crate root (correct asset!() macro syntax)
    let root_dir = root_dir();
    let rel_path = cached_asset_path.strip_prefix(&root_dir).unwrap();
    let path = format!("/{}", rel_path.display());

    // insert asset call
    quote! {
        asset!(#path)
    }
}


/// downloads the asset at the given URL and caches it at *compile time*, and returns a dioxus
/// `asset!()` macro call, embedding the asset for offline/self-hosting use at runtime
///
/// # Arguments
///
/// * `input`: a string of a valid URL pointing to a file that will be downloaded to a file and served via `asset!()`
///
/// # Examples
///
/// ```
/// use dioxus::prelude::*;
/// use dioxus_google_font_embedder::asset_url;
///
/// fn main() {
///     dioxus::launch(App);
/// }
///
/// #[component]
/// fn App() -> Element {
///     rsx! {
///         Stylesheet { href: asset_url!("https://cdn.jsdelivr.net/npm/bootstrap@latest/dist/css/bootstrap.min.css") }
///         p {
///             "This page will have the Bootstrap CSS embedded at compile time, and will work offline!"
///         }
///     }
/// }
/// ```
#[proc_macro]
pub fn asset_url(input: TokenStream) -> TokenStream {
    let url = syn::parse_macro_input!(input as LitStr).value();

    expand_asset_url(&url).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_output() {
        println!("{}", expand_google_font(
            "https://fonts.googleapis.com/css2?family=Atkinson+Hyperlegible+Mono:ital,wght@0,200..800;1,200..800&family=Atkinson+Hyperlegible+Next:ital,wght@0,200..800;1,200..800&display=swap",
        ));
        println!("{}", expand_asset_url(
            "https://cdn.jsdelivr.net/npm/bootstrap@latest/dist/css/bootstrap.min.css",
        ));
    }
}
