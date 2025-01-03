#![allow(dead_code)]

mod dirs;
mod server;

use server::authn::token::config::TokenConfig;

use crate::server::authn::token::factory::TokenizerFactory;

fn main() {
    let cfg = TokenConfig {
        pki_path: "testdata/config/pki".into(),
        expiry: 12,
        no_generate_keys: false,
    };
    let tokenizer = TokenizerFactory::new().build_tokenizer(&cfg).unwrap();
    let token = tokenizer
        .generate_token("test_user".to_string(), 1)
        .unwrap();
    println!("{token}");
    println!("Hello, world!");
}
