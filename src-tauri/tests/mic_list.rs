//! Shows the microphone list exactly as Settings will render it.
#[test]
fn print_microphones() {
    match kiku_lib::audio::list_microphones() {
        Ok(list) => {
            println!("  microphones offered: {}", list.len());
            for m in &list {
                println!(
                    "    {}{}",
                    m.name,
                    if m.is_default { "   (default)" } else { "" }
                );
            }
        }
        Err(e) => println!("  error: {e}"),
    }
}
