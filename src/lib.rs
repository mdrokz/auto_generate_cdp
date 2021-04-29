extern crate proc_macro2;

use quote::quote;

use std::{fs::OpenOptions, process::Command};

use std::io::prelude::*;

mod types;

mod compile;

use crate::compile::compile_cdp_json;

pub fn init() {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(format!("./src/protocol.rs"))
        .unwrap();

    file.sync_all().unwrap();

    if file.metadata().unwrap().len() <= 0 {
        let js_mods = compile_cdp_json("./js_protocol.json");

        let browser_mods = compile_cdp_json("./browser_protocol.json");

        let modv = quote! {
            pub mod cdp {

                pub mod types {
                    use serde::{Deserialize, Serialize};
                    use std::fmt::Debug;

                    pub type JsInt = i32;
                    pub type JsUInt = u32;

                    pub type WindowId = JsUInt;

                    pub type CallId = JsUInt;
                    

                    #[derive(Serialize, Debug)]
                    pub struct MethodCall<T>
                    where
                    T: Debug,
                    {
                        #[serde(rename = "method")]
                        method_name: &'static str,
                        pub id: CallId,
                        params: T,
                    }

                    impl<T> MethodCall<T>
                    where
                    T: Debug,
                    {
                        pub fn get_params(&self) -> &T {
                        &self.params
                        }
                    }

                    pub trait Method: Debug {
                    const NAME: &'static str;

                    type ReturnObject: serde::de::DeserializeOwned + std::fmt::Debug;


                    fn to_method_call(self, call_id: CallId) -> MethodCall<Self>
                    where
                    Self: std::marker::Sized,
                    {
                        MethodCall {
                            id: call_id,
                             params: self,
                            method_name: Self::NAME,
                            }
                    }

                    }
                }

                #(#js_mods)*
                #(#browser_mods)*
            }
        };

        writeln!(file, "{}", modv.to_string()).unwrap();

        Command::new("rustfmt")
            .arg("./src/protocol.rs")
            .output()
            .unwrap();
    }
}
