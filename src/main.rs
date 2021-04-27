extern crate proc_macro2;

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

use std::{fs, iter::FromIterator};

use std::io::prelude::*;

use convert_case::{Case, Casing};

mod types;

mod test;

use fs::OpenOptions;
use types::{Domain, Parameter, TypeElement, TypeEnum};

use crate::types::Protocol;

pub trait StringUtils {
    fn first_uppercase(&mut self);
}

impl StringUtils for String {
    fn first_uppercase(&mut self) {
        let s = self
            .chars()
            .enumerate()
            .map(|(i, f)| if i == 0 { f.to_ascii_uppercase() } else { f })
            .collect::<Vec<char>>();

        self.clone_from(&String::from_iter(s));
    }
}

impl Into<Option<Ident>> for TypeEnum {
    fn into(self) -> Option<Ident> {
        match self {
            TypeEnum::Boolean => Some(Ident::new("bool", Span::call_site())),
            TypeEnum::Integer => Some(Ident::new("JsInt", Span::call_site())),
            TypeEnum::Number => Some(Ident::new("JsUint", Span::call_site())),
            TypeEnum::String => Some(Ident::new("String", Span::call_site())),
            TypeEnum::Any => Some(Ident::new("Json", Span::call_site())),
            _ => None,
        }
    }
}

enum PropertyType<'a> {
    Param(&'a Parameter),
    Element(&'a TypeElement),
}

fn get_types(
    type_type: TypeEnum,
    property_type: PropertyType,
    types: &mut Vec<TokenStream>,
    enums: &mut Vec<TokenStream>,
    objects: &mut Vec<TokenStream>,
    object: &mut Vec<TokenStream>,
    dependencies: &mut Vec<TokenStream>,
    previous_type: Option<Ident>,
) {
    match property_type {
        PropertyType::Param(param) => {
            let mut name = Ident::new(&String::from(param.name.clone()), Span::call_site());
            if param.name == "type" {
                name = Ident::new(&String::from("Type"), Span::call_site());
            }
            match type_type {
                TypeEnum::Array => {
                    let items = param.items.as_ref().unwrap();

                    if let Some(p_ref) = &items.items_ref {
                        let v = quote! {
                            pub #name: #previous_type<#p_ref>,
                        };
                        object.push(v);
                    } else {
                        let p_type = items.items_type.as_ref().unwrap().clone();

                        get_types(
                            p_type,
                            PropertyType::Param(param),
                            types,
                            enums,
                            objects,
                            object,
                            dependencies,
                            Some(Ident::new("Vec", Span::call_site())),
                        );
                    }
                }
                _ => {
                    let type_type: Option<Ident> = type_type.into();
                    let type_type = type_type.unwrap();
                    if let Some(p_type) = previous_type {
                        let v = quote! {
                            pub #name: #p_type<#type_type>,
                        };
                        object.push(v);
                    } else {
                        let v = quote! {
                            pub #name: #type_type,
                        };
                        object.push(v);
                    }
                }
            }
        }
        PropertyType::Element(type_element) => {
            let name = Ident::new(&type_element.id, Span::call_site());

            match type_type {
                TypeEnum::Array => {
                    let items = type_element.items.as_ref().unwrap();

                    if let Some(p_ref) = &items.items_ref {
                        let v = quote! {
                            type #name = Vec<#p_ref>;
                        };
                        types.push(v);
                    } else {
                        let p_type = items.items_type.as_ref().unwrap().clone();

                        get_types(
                            p_type,
                            PropertyType::Element(type_element),
                            types,
                            enums,
                            objects,
                            object,
                            dependencies,
                            Some(Ident::new("Vec", Span::call_site())),
                        );
                    }
                }
                TypeEnum::Object => {
                    if let Some(properties) = type_element.properties.as_deref() {
                        for property in properties {
                            // println!("{:?}", property);
                            match &property.parameter_type {
                                Some(p) => get_types(
                                    p.clone(),
                                    PropertyType::Param(property),
                                    types,
                                    enums,
                                    objects,
                                    object,
                                    dependencies,
                                    None,
                                ),
                                None => {
                                    let name = Ident::new(&property.name, Span::call_site());
                                    let mut p_ref =
                                        property.parameter_ref.as_ref().unwrap().clone();

                                    if p_ref.contains(".") {
                                        let dep = &p_ref
                                            .split(".")
                                            .map(|v| Ident::new(v, Span::call_site()))
                                            .collect::<Vec<Ident>>()[0];

                                        dependencies.push(quote! {
                                            use super::#dep;
                                        });
                                    }

                                    if p_ref == type_element.id {
                                        let p_ref = Ident::new(&p_ref, Span::call_site());
                                        let v = quote! {
                                            pub #name: Box<#p_ref>,
                                        };
                                        object.push(v);
                                    } else {
                                        let dep = p_ref
                                            .split(".")
                                            .map(|v| Ident::new(v, Span::call_site()))
                                            .collect::<Vec<Ident>>();

                                        let v = quote! {
                                            pub #name: #(#dep)::*,
                                        };
                                        object.push(v);
                                    }
                                }
                            };
                        }
                    } else {
                        let v = quote! {
                            type #name = serde_json::Value;
                        };
                        types.push(v);
                    }
                    objects.push(quote! {
                            #[derive(Serialize, Debug)]
                            #[serde(rename_all = "camelCase")]
                            pub struct #name {
                                #(#object)*
                            }
                    });
                }
                TypeEnum::String => {
                    if let Some(enum_vec) = type_element.type_enum.clone() {
                        let mut enum_tokens: Vec<TokenStream> = vec![];

                        for e in enum_vec {
                            if e.contains("-") {
                                let enum_type = e
                                    .split("-")
                                    .map(|s| {
                                        let mut upper = s.to_string();
                                        upper.first_uppercase();

                                        upper
                                    })
                                    .collect::<Vec<String>>()
                                    .join("");

                                let enum_type = Ident::new(&enum_type, Span::call_site());

                                enum_tokens.push(quote! {
                                    #enum_type,
                                });
                            } else {
                                let enum_type =
                                    Ident::new(&e.to_case(Case::Pascal), Span::call_site());
                                enum_tokens.push(quote! {
                                    #enum_type,
                                });
                            }
                        }

                        let typ_enum = quote! {
                            #[derive(Serialize, Debug)]
                            #[serde(rename_all = "camelCase")]
                            pub enum #name {
                                #(#enum_tokens)*
                            }
                        };

                        enums.push(typ_enum);
                    } else {
                        if let Some(p_type) = previous_type {
                            let v = quote! {
                                type #name = #p_type<String>;
                            };

                            types.push(v);
                        } else {
                            let v = quote! {
                                type #name = String;
                            };

                            types.push(v);
                        }
                    }
                }
                _ => {
                    let type_type: Option<Ident> = type_type.into();
                    let type_type = type_type.unwrap();
                    if let Some(p_type) = previous_type {
                        let v = quote! {
                            type #name = #p_type<#type_type>;
                        };
                        types.push(v);
                    } else {
                        let v = quote! {
                            type #name = #type_type;
                        };
                        types.push(v);
                    }
                }
            }
        }
    };
}

fn main() {
    let json = fs::read_to_string("./browser_protocol.json").unwrap();

    let protocol: Protocol = serde_json::from_str(&json).unwrap();

    let doms = protocol
        .domains
        .iter()
        .filter(|d| &d.domain == "DOM")
        .collect::<Vec<&Domain>>();

    for dom in doms {
        let mut types = Vec::new();
        let mut enums = Vec::new();
        let mut objects = Vec::new();
        let mut dependencies = Vec::new();

        let mut command_objects = String::new();

        let mut parameter_objects = String::new();

        for dep in dom
            .dependencies
            .as_ref()
            .unwrap()
            .iter()
            .map(|v| Ident::new(&v.trim(), Span::call_site()))
            .collect::<Vec<Ident>>()
        {
            dependencies.push(quote! {
                use super::#dep;
            });
        }

        if let Some(type_elements) = dom.types.as_deref() {
            for type_element in type_elements {
                get_types(
                    type_element.type_type,
                    PropertyType::Element(type_element),
                    &mut types,
                    &mut enums,
                    &mut objects,
                    &mut Vec::new(),
                    &mut dependencies,
                    None,
                );
            }
        }

        let name = Ident::new(&dom.domain, Span::call_site());

        /*
                        #(#types)*

                #(#enums)*

                #(#objects)*
        */
        let m = quote! {
            pub mod #name {

                use serde_json::Value as Json;

                #(#dependencies)*

            }
        };

        println!(
            "{}",
            quote! {
                #m
            }
            .to_string()
        );
    }

    //     for command in &dom.commands {
    //         if let Some(returns) = &command.returns {
    //             let mut name = command.name.clone();
    //             name.first_uppercase();
    //             let mut object = String::from(format!(
    //                 r#" #[derive(Serialize, Debug)] #[serde(rename_all = "camelCase")] pub struct {}{}"#,
    //                 name, "ReturnObject"
    //             ));
    //             object.push_str("\n{");
    //             for return_type in returns {
    //                 if let Some(param_type) = return_type.parameter_type {
    //                     let name = return_type.name.clone();
    //                     match param_type {
    //                         TypeEnum::Array => {
    //                             let items = return_type.items.as_ref().unwrap();
    //                             if let Some(ref_type) = items.items_ref.clone() {
    //                                 object.push_str(&format!("pub {}:Vec<{}>,", name, ref_type));
    //                             } else {
    //                                 let type_type: &str = items.items_type.unwrap().into();

    //                                 object.push_str(&format!("pub {}:Vec<{}>,", name, type_type));
    //                             }
    //                         }
    //                         TypeEnum::String => {
    //                             object.push_str(&format!("\n pub {}:{},", name, "String"));
    //                         }
    //                         TypeEnum::Boolean => {
    //                             object.push_str(&format!("\n pub {}:{},", name, "bool"));
    //                         }
    //                         TypeEnum::Number => {
    //                             object.push_str(&format!("\n pub {}:{},", name, "JsInt"));
    //                         }
    //                         TypeEnum::Integer => {
    //                             object.push_str(&format!("\n pub {}:{},", name, "JsUInt"));
    //                         }
    //                         _ => {
    //                             object.push_str(&format!("\n pub {}:{},", name, "JsUInt"));
    //                         }
    //                     }
    //                 } else {
    //                     let p_ref = return_type.parameter_ref.clone().unwrap();
    //                     object.push_str(&format!("\n pub {}:{},", return_type.name, p_ref));
    //                 }
    //             }
    //             object.push_str("}");
    //             command_objects.push_str(&object);
    //         } else {
    //             let mut name = command.name.clone();
    //             name.first_uppercase();
    //             let mut object = String::from(format!(
    //                 r#" #[derive(Serialize, Debug)] #[serde(rename_all = "camelCase")] pub struct {}{}"#,
    //                 name, "ReturnObject"
    //             ));

    //             object.push_str("{}");

    //             command_objects.push_str(&object);
    //         }

    //         if let Some(parameters) = command.parameters.as_deref() {
    //             let mut name = command.name.clone();
    //             name.first_uppercase();
    //             let mut object = String::from(format!(
    //                 r#" #[derive(Serialize, Debug)] #[serde(rename_all = "camelCase")] pub struct {}"#,
    //                 name
    //             ));
    //             object.push_str("\n{");
    //             for parameter in parameters {
    //                 if let Some(param_type) = parameter.parameter_type {
    //                     let name = parameter.name.clone();
    //                     match param_type {
    //                         TypeEnum::Array => {
    //                             let items = parameter.items.as_ref().unwrap();
    //                             if let Some(ref_type) = items.items_ref.clone() {
    //                                 object.push_str(&format!("pub {}:Vec<{}>,", name, ref_type));
    //                             } else {
    //                                 let type_type: &str = items.items_type.unwrap().into();

    //                                 object.push_str(&format!("pub {}:Vec<{}>,", name, type_type));
    //                             }
    //                         }
    //                         TypeEnum::String => {
    //                             object.push_str(&format!("\n pub {}:{},", name, "String"));
    //                         }
    //                         TypeEnum::Boolean => {
    //                             object.push_str(&format!("\n pub {}:{},", name, "bool"));
    //                         }
    //                         TypeEnum::Number => {
    //                             object.push_str(&format!("\n pub {}:{},", name, "JsInt"));
    //                         }
    //                         TypeEnum::Integer => {
    //                             object.push_str(&format!("\n pub {}:{},", name, "JsUInt"));
    //                         }
    //                         _ => {}
    //                     }
    //                 } else {
    //                     let p_ref = parameter.parameter_ref.clone().unwrap();
    //                     object.push_str(&format!("\n pub {}:{},", parameter.name, p_ref));
    //                 }
    //             }
    //             object.push_str("}");
    //             parameter_objects.push_str(&object);
    //         } else {
    //             let mut name = command.name.clone();
    //             name.first_uppercase();
    //             let mut object = String::from(format!(
    //                 r#" #[derive(Serialize, Debug)] #[serde(rename_all = "camelCase")] pub struct {}"#,
    //                 name,
    //             ));

    //             object.push_str("{}");

    //             parameter_objects.push_str(&object);
    //         }
    //     }

    //     let mut file = OpenOptions::new()
    //         .append(true)
    //         .create(true)
    //         .open(format!("./src/{}.rs", dom.domain))
    //         .unwrap();

    //     writeln!(file, "{}", types).unwrap();
    //     writeln!(file, "{}", enums).unwrap();
    //     writeln!(file, "{}", objects).unwrap();
    //     writeln!(file, "{}", parameter_objects).unwrap();
    //     writeln!(file, "{}", command_objects).unwrap();
    // }
}
