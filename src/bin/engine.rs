// src/bin/engine.rs

fn main() {
    cshogi_rust::attacks::init_attack_tables();
    cshogi_rust::usi::usi_loop();
}
