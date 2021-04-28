extern crate proc_macro2;

use quote::quote;

use std::{fs};

use std::io::prelude::*;

mod types;

mod compile;

use fs::OpenOptions;

use crate::compile::compile_cdp_json;

fn main() {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(format!("./src/protocol.rs"))
        .unwrap();

    let js_mods = compile_cdp_json("./js_protocol.json");

    let browser_mods = compile_cdp_json("./browser_protocol.json");

    let modv = quote! {
        pub mod cdp {

            mod types {
                pub type JsInt = i32;
                pub type JsUInt = u32;
            }

            #(#js_mods)*
            #(#browser_mods)*
        }
    };

    writeln!(file, "{}", modv.to_string()).unwrap();
}
