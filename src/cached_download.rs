use std::fs;
use std::path::{Path, PathBuf};
use url::Url;

pub(crate) fn cached_download(url: &str, cache_dir: &PathBuf) -> PathBuf {
    let hash = format!("{:x}", md5::compute(url));

    // 1. Check if we already have a cached file matching this URL's hash.
    if let Ok(entries) = fs::read_dir(cache_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(stem) = path.file_stem() {
                if stem.to_string_lossy() == hash {
                    return path; // Found the cached file
                }
            }
        }
    }

    eprintln!("dioxus_google_font_embedder: URL {url} not cached, downloading and caching...");

    // 2. Download the file
    let resp = ureq::get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .call()
        .unwrap();

    // 3. Try to get the extension cleanly from the URL path first
    let parsed_url = Url::parse(url).unwrap();
    let url_path = parsed_url.path(); // This safely strips off ?query=params

    let ext_from_url = Path::new(url_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_string());

    // 4. Determine the final extension (URL first, MIME type fallback)
    let extension = ext_from_url.unwrap_or_else(|| {
        let content_type_header = resp
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("application/octet-stream");

        let mime_type = content_type_header.split(';').next().unwrap().trim();

        mime_guess::get_mime_extensions_str(mime_type)
            .and_then(|exts| exts.first().copied())
            .unwrap_or("bin")
            .to_string()
    });

    // 5. Save to the final path
    let file_path = cache_dir.join(format!("{hash}.{extension}"));
    let mut file = fs::File::create(&file_path).unwrap();

    std::io::copy(&mut resp.into_body().into_reader(), &mut file).unwrap();

    file_path
}