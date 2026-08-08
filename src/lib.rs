#![feature(str_as_str)]

mod googlefontsapi;

use crate::googlefontsapi::{cached_download};
use proc_macro::TokenStream;
use std::fs;
use std::path::PathBuf;
use quote::quote;
use regex::Regex;
use syn::LitStr;


fn cache_dir() -> PathBuf {
    // appropriate place to save files
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let cache_dir = std::path::Path::new(&manifest_dir).join("target").join("google-fonts-cache");
    fs::create_dir_all(&cache_dir).unwrap();
    cache_dir
}


fn expand(url: &str) -> proc_macro2::TokenStream {
    let cache_dir = cache_dir();

    let csspath = cached_download(url, &cache_dir);

    let css = fs::read_to_string(&csspath).unwrap();

    // css url(): in this case, font files!
    // claude wrote this regex cause who cares
    let re = Regex::new(r#"url\(['"]?(https?://[^)'"]+)['"]?\)"#).unwrap();

    // prep to be used as a format string, escape bracket literals
    let css = css.replace("{", "{{").replace("}", "}}");

    let mut urls = Vec::<String>::new();

    // for every url (which is a font we must download), save to a list, and prep to be re-inserted via format!
    let css = re.replace_all(&*css, |caps: &regex::Captures| {
        let original_url = &caps[1];
        urls.push(original_url.to_string());
        //
        "url({})"
    });

    let csshash = format!("{:x}.css", md5::compute(&*css));
    let css_format_file = cache_dir.join(&csshash).into_os_string().into_string().unwrap();

    fs::write(&css_format_file, css.as_str()).unwrap();
    // dbg!(&urls, &css);

    let paths = urls.iter().map(|url| {
        // let filename = url.rsplit('/').next().unwrap();
        // let path = cache_dir.join(filename);
        cached_download(url, &cache_dir)
    }).collect::<Vec<String>>();

    quote! {
        rsx! {
            style {
                { format!(include_str!(#css_format_file), #(asset!(#paths)),*) }
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
        let out = expand("https://fonts.googleapis.com/css2?family=Atkinson+Hyperlegible+Mono:ital,wght@0,200..800;1,200..800&family=Atkinson+Hyperlegible+Next:ital,wght@0,200..800;1,200..800&display=swap");
        println!("{}", out);
    }
}