extern crate proc_macro2;

use quote::quote;

use std::{fs::OpenOptions, process::Command};

use std::io::prelude::*;

mod types;

mod compile;

use crate::compile::compile_cdp_json;

pub fn init() {
    const CDP_COMMIT: &str = "15f524c8f5ce5b317ddcdf5e6f875d6eb8bdac88";
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(format!("./src/protocol.rs"))
        .unwrap();

    file.sync_all().unwrap();

    if file.metadata().unwrap().len() <= 0 {
        let (js_mods,js_events) =
            compile_cdp_json("./js_protocol.json", CDP_COMMIT);

        let (browser_mods,browser_events) =
            compile_cdp_json("./browser_protocol.json", CDP_COMMIT);

        writeln!(file, "// Auto-generated from ChromeDevTools/devtools-protocol at commit {}", CDP_COMMIT).unwrap();

        let modv = quote! {
            #[allow(unused)]
            #[allow(non_camel_case_types)]
            #[allow(non_snake_case)]
            
            pub mod cdp {

                pub mod types {
                    use serde::{Deserialize, Serialize};
                    use std::fmt::Debug;

                    pub type JsFloat = f64;
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

                    #[derive(Deserialize, Debug, Clone, PartialEq)]
                    #[serde(tag = "method")]
                    #[allow(clippy::large_enum_variant)]
                    pub enum Event {
                        #(#browser_events)*
                        #(#js_events)*
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

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        crate::init();
    }
}