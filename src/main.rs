#[cfg(target_arch = "wasm32")]
fn main() {
    yew::Renderer::<ponzisim::ui::App>::new().render();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("Ponzisim UI targets WebAssembly. Run with: trunk serve --open");
}
