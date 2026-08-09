use std::path::PathBuf;

pub(crate) fn download_to_file(url: &str, path: &PathBuf) {
    let resp = ureq::get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .call()
        .unwrap();

    let mut file = std::fs::File::create(path).unwrap();
    std::io::copy(&mut resp.into_body().into_reader(), &mut file).unwrap();
}

pub(crate) fn cached_download(url: &str, cache_dir: &PathBuf) -> PathBuf {
    let extension = url.rsplit('.').next().unwrap();
    let hash = format!("{:x}.{extension}", md5::compute(url));
    let file = cache_dir.join(&hash);

    if !file.exists() {
        eprintln!("dioxus_google_font_embedder: URL {url} not cached, downloading and caching...");
        download_to_file(url, &file);
    }
    file
}