use dioxus::prelude::*;
use dioxus_google_font_embedder::{embed_google_font, asset_url};

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        Stylesheet { href: asset_url!("https://cdn.jsdelivr.net/npm/bootstrap@latest/dist/css/bootstrap.min.css") }
        {embed_google_font!("https://fonts.googleapis.com/css2?family=Atkinson+Hyperlegible+Next:ital,wght@0,200..800;1,200..800&display=swap")}
        p {
            font_family: "Atkinson Hyperlegible Next",
            "This text will be rendered using a Google Font that is downloaded, cached, and embedded at compile time!"
        }
    }
}
