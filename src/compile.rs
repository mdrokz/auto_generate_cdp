use std::iter::FromIterator;

use convert_case::{Case, Casing};

use crate::types::{Command, Event, Parameter, Protocol, TypeElement, TypeEnum};

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

pub trait StringUtils {
    fn first_uppercase(&mut self);
    fn replace_if<F>(self, from: &str, to: &str, predicate: F) -> String
    where
        F: FnOnce() -> bool;
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

    fn replace_if<F>(self, from: &str, to: &str, predicate: F) -> String
    where
        F: FnOnce() -> bool,
    {
        if predicate() {
            self.replace(from, to)
        } else {
            self.clone()
        }
    }
}

impl Into<Option<Ident>> for TypeEnum {
    fn into(self) -> Option<Ident> {
        match self {
            TypeEnum::Boolean => Some(Ident::new("bool", Span::call_site())),
            TypeEnum::Integer => Some(Ident::new("JsUInt", Span::call_site())),
            TypeEnum::Number => Some(Ident::new("JsFloat", Span::call_site())),
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
    type_element: Option<&TypeElement>,
    types: &mut Vec<TokenStream>,
    enums: &mut Vec<TokenStream>,
    objects: &mut Vec<TokenStream>,
    object: &mut Vec<TokenStream>,
    dependencies: &mut Vec<TokenStream>,
    previous_type: Option<Ident>,
) {
    match property_type {
        PropertyType::Param(param) => {
            // || param.name.starts_with("type")
            let name = Ident::new(
                &String::from(param.name.clone().to_case(Case::Snake).replace_if(
                    "type",
                    "Type",
                    || param.name.starts_with("type"),
                )),
                Span::call_site(),
            );

            match type_type {
                TypeEnum::Array => {
                    let items = param.items.as_ref().unwrap();

                    if let Some(p_ref) = &items.items_ref {
                        if let Some(p_type) = previous_type {
                            if let Some(_) = param.optional {
                                let v = quote! {
                                    #[serde(skip_serializing_if="Option::is_none")]
                                    pub #name: Option<#p_type<#p_ref>>,
                                };
                                object.push(v);
                            } else {
                                let v = quote! {
                                    pub #name: #p_type<#p_ref>,
                                };
                                object.push(v);
                            }
                        } else {
                            if p_ref.contains(".") {
                                let dep = &p_ref
                                    .split(".")
                                    .map(|v| Ident::new(v, Span::call_site()))
                                    .collect::<Vec<Ident>>()[0];

                                let v: Vec<&TokenStream> = dependencies
                                    .iter()
                                    .filter(|v| {
                                        let r = p_ref.split(".").collect::<Vec<&str>>()[0];
                                        v.to_string().contains(r)
                                    })
                                    .collect();

                                if v.len() <= 0 {
                                    dependencies.push(quote! {
                                        use super::#dep;
                                    });
                                }
                            }

                            let dep = p_ref
                                .split(".")
                                .map(|v| Ident::new(v, Span::call_site()))
                                .collect::<Vec<Ident>>();

                            if let Some(_) = param.optional {
                                let v = quote! {
                                    #[serde(skip_serializing_if="Option::is_none")]
                                    pub #name: Option<Vec<#(#dep)::*>>,
                                };
                                object.push(v);
                            } else {
                                let v = quote! {
                                    pub #name: Vec<#(#dep)::*>,
                                };
                                object.push(v);
                            }
                        }
                    } else {
                        let p_type = items.items_type.as_ref().unwrap().clone();

                        get_types(
                            p_type,
                            PropertyType::Param(param),
                            type_element,
                            types,
                            enums,
                            objects,
                            object,
                            dependencies,
                            Some(Ident::new("Vec", Span::call_site())),
                        );
                    }
                }
                TypeEnum::String => {
                    if let Some(enum_vec) = param.parameter_enum.clone() {
                        let mut enum_tokens: Vec<TokenStream> = vec![];

                        for e in enum_vec.clone() {
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
                        let element_name = type_element.unwrap();

                        let mut enum_name = String::new();

                        enum_name = String::from(element_name.id.clone());
                        enum_name.push_str(&name.to_string().to_case(Case::Pascal));
                        enum_name = enum_name.to_case(Case::Pascal);

                        enum_name.first_uppercase();

                        let enum_name = Ident::new(&enum_name, Span::call_site());

                        let vec: Vec<&String> = enum_vec
                            .iter()
                            .filter(|v| {
                                let c = v.chars().next().unwrap();
                                if c.is_uppercase() {
                                    true
                                } else if v.contains("-") {
                                    true
                                } else {
                                    false
                                }
                            })
                            .collect();

                        let mut rename = quote! {
                            #[serde(rename_all = "camelCase")]
                        };
                        if vec.len() > 0 {
                            let v = vec[0];

                            let c = v.chars().next().unwrap();

                            if c.is_uppercase() {
                                rename = quote! {
                                #[serde(rename_all = "PascalCase")]
                                }
                            } else if v.contains("-") {
                                rename = quote! {
                                #[serde(rename_all = "kebab-case")]
                                }
                            }
                        }

                        let typ_enum = quote! {
                            #[derive(Deserialize,Serialize, Debug,Clone,PartialEq)]
                            #rename
                            pub enum #enum_name {
                                #(#enum_tokens)*
                            }
                        };

                        if let Some(p_type) = previous_type {
                            if let Some(_) = param.optional {
                                let v = quote! {
                                    #[serde(skip_serializing_if="Option::is_none")]
                                    pub #name: Option<#p_type<#enum_name>>,
                                };
                                object.push(v);
                            } else {
                                let v = quote! {
                                    pub #name: #p_type<#enum_name>,
                                };
                                object.push(v);
                            }
                        } else {
                            if let Some(_) = param.optional {
                                let v = quote! {
                                    #[serde(skip_serializing_if="Option::is_none")]
                                    pub #name: Option<#enum_name>,
                                };
                                object.push(v);
                            } else {
                                let v = quote! {
                                    pub #name: #enum_name,
                                };
                                object.push(v);
                            }
                        }

                        enums.push(typ_enum);
                    } else {
                        if let Some(p_type) = previous_type {
                            if let Some(_) = param.optional {
                                let v = quote! {
                                    #[serde(skip_serializing_if="Option::is_none")]
                                    pub #name: Option<#p_type<String>>,
                                };
                                object.push(v);
                            } else {
                                let v = quote! {
                                    pub #name: #p_type<String>,
                                };
                                object.push(v);
                            }
                        } else {
                            if let Some(_) = param.optional {
                                let v = quote! {
                                    #[serde(skip_serializing_if="Option::is_none")]
                                    pub #name: Option<String>,
                                };
                                object.push(v);
                            } else {
                                let v = quote! {
                                    pub #name: String,
                                };
                                object.push(v);
                            }
                        }
                    }
                }
                _ => {
                    let mut type_type: Option<Ident> = type_type.into();

                    if let Some(typ) = type_type {
                        if let Some(p_type) = previous_type {
                            if let Some(_) = param.optional {
                                let v = quote! {
                                    #[serde(skip_serializing_if="Option::is_none")]
                                    pub #name: Option<#p_type<#typ>>,
                                };
                                object.push(v);
                            } else {
                                let v = quote! {
                                    pub #name: #p_type<#typ>,
                                };
                                object.push(v);
                            }
                        } else {
                            if let Some(_) = param.optional {
                                let v = quote! {
                                    #[serde(skip_serializing_if="Option::is_none")]
                                    pub #name: Option<#typ>,
                                };
                                object.push(v);
                            } else {
                                let v = quote! {
                                    pub #name: #typ,
                                };
                                object.push(v);
                            }
                        }
                    }
                }
            }
        }
        PropertyType::Element(typ_element) => {
            let name = Ident::new(&typ_element.id, Span::call_site());

            match type_type {
                TypeEnum::Array => {
                    let items = typ_element.items.as_ref().unwrap();

                    if let Some(p_ref) = &items.items_ref {
                        let p_ref = Ident::new(&p_ref, Span::call_site());
                        let v = quote! {
                            pub type #name = Vec<#p_ref>;
                        };
                        types.push(v);
                    } else {
                        let p_type = items.items_type.as_ref().unwrap().clone();

                        get_types(
                            p_type,
                            PropertyType::Element(typ_element),
                            Some(typ_element.clone()),
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
                    if let Some(properties) = typ_element.properties.as_deref() {
                        for property in properties {
                            // println!("{:?}", property);
                            match &property.parameter_type {
                                Some(p) => get_types(
                                    p.clone(),
                                    PropertyType::Param(property),
                                    Some(typ_element.clone()),
                                    types,
                                    enums,
                                    objects,
                                    object,
                                    dependencies,
                                    None,
                                ),
                                None => {
                                    let p_name = Ident::new(
                                        &property.name.to_case(Case::Snake).replace("type", "Type"),
                                        Span::call_site(),
                                    );

                                    let p_ref = property.parameter_ref.as_ref().unwrap().clone();

                                    if p_ref.contains(".") {
                                        let dep = &p_ref
                                            .split(".")
                                            .map(|v| Ident::new(v, Span::call_site()))
                                            .collect::<Vec<Ident>>()[0];

                                        let v: Vec<&TokenStream> = dependencies
                                            .iter()
                                            .filter(|v| {
                                                let r = p_ref.split(".").collect::<Vec<&str>>()[0];
                                                v.to_string().contains(r)
                                            })
                                            .collect();

                                        if v.len() <= 0 {
                                            dependencies.push(quote! {
                                                use super::#dep;
                                            });
                                        }
                                    }

                                    if p_ref == typ_element.id {
                                        let p_ref = Ident::new(&p_ref, Span::call_site());
                                        if let Some(_) = property.optional {
                                            let v = quote! {
                                                #[serde(skip_serializing_if="Option::is_none")]
                                                pub #p_name: Option<Box<#p_ref>>,
                                            };
                                            object.push(v);
                                        } else {
                                            let v = quote! {
                                                pub #p_name: Box<#p_ref>,
                                            };
                                            object.push(v);
                                        }
                                    } else {
                                        let dep = p_ref
                                            .split(".")
                                            .map(|v| Ident::new(v, Span::call_site()))
                                            .collect::<Vec<Ident>>();

                                        if let Some(_) = property.optional {
                                            let v = quote! {
                                                #[serde(skip_serializing_if="Option::is_none")]
                                                pub #p_name: Option<#(#dep)::*>,
                                            };
                                            object.push(v);
                                        } else {
                                            let v = quote! {
                                                pub #p_name: #(#dep)::*,
                                            };
                                            object.push(v);
                                        }
                                    }
                                }
                            };
                        }
                    }
                    objects.push(quote! {
                            #[derive(Deserialize,Serialize, Debug,Clone,PartialEq)]
                            #[serde(rename_all = "camelCase")]
                            pub struct #name {
                                #(#object)*
                            }
                    });
                }
                TypeEnum::String => {
                    if let Some(enum_vec) = typ_element.type_enum.clone() {
                        let mut enum_tokens: Vec<TokenStream> = vec![];

                        for e in enum_vec.clone() {
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

                        let vec: Vec<&String> = enum_vec
                            .iter()
                            .filter(|v| {
                                let c = v.chars().next().unwrap();
                                if c.is_uppercase() {
                                    true
                                } else if v.contains("-") {
                                    true
                                } else {
                                    false
                                }
                            })
                            .collect();

                        let mut rename = quote! {
                            #[serde(rename_all = "camelCase")]
                        };
                        if vec.len() > 0 {
                            let v = vec[0];

                            let c = v.chars().next().unwrap();

                            if c.is_uppercase() {
                                rename = quote! {
                                #[serde(rename_all = "PascalCase")]
                                }
                            } else if v.contains("-") {
                                rename = quote! {
                                #[serde(rename_all = "kebab-case")]
                                }
                            }
                        }

                        let typ_enum = quote! {
                            #[derive(Deserialize,Serialize, Debug,Clone,PartialEq)]
                            #rename
                            pub enum #name {
                                #(#enum_tokens)*
                            }
                        };

                        enums.push(typ_enum);
                    } else {
                        if let Some(p_type) = previous_type {
                            let v = quote! {
                                pub type #name = #p_type<String>;
                            };

                            types.push(v);
                        } else {
                            let v = quote! {
                                pub type #name = String;
                            };

                            types.push(v);
                        }
                    }
                }
                _ => {
                    let mut type_type: Option<Ident> = type_type.into();

                    if let Some(typ) = type_type {
                        if let Some(p_type) = previous_type {
                            let v = quote! {
                                pub type #name = #p_type<#typ>;
                            };
                            types.push(v);
                        } else {
                            let v = quote! {
                                pub type #name = #typ;
                            };
                            types.push(v);
                        }
                    }
                }
            }
        }
    };
}

pub fn get_commands(
    commands: &Vec<Command>,
    dependencies: &mut Vec<TokenStream>,
    command_objects: &mut Vec<TokenStream>,
    parameter_objects: &mut Vec<TokenStream>,
    enums: &mut Vec<TokenStream>,
) {
    for command in commands {
        let mut name = command.name.clone();
        name.first_uppercase();
        name.push_str("ReturnObject");
        let name = Ident::new(&name, Span::call_site());
        if let Some(returns) = &command.returns {
            let mut command_object: Vec<TokenStream> = Vec::new();

            for return_type in returns {
                if let Some(param_type) = return_type.parameter_type {
                    let name = Ident::new(
                        &return_type.name.clone().to_case(Case::Snake),
                        Span::call_site(),
                    );

                    match param_type {
                        TypeEnum::Array => {
                            let items = return_type.items.as_ref().unwrap();

                            if let Some(ref_type) = items.items_ref.clone() {
                                if ref_type.contains(".") {
                                    let dep = ref_type
                                        .split(".")
                                        .map(|v| Ident::new(v, Span::call_site()))
                                        .collect::<Vec<Ident>>();

                                    if let Some(_) = return_type.optional {
                                        let v = quote! {
                                            #[serde(skip_serializing_if="Option::is_none")]
                                            pub #name: Option<#(#dep)::*>,
                                        };

                                        command_object.push(v);
                                    } else {
                                        let v = quote! {
                                            pub #name: #(#dep)::*,
                                        };

                                        command_object.push(v);
                                    }
                                } else {
                                    let ref_type = Ident::new(&ref_type, Span::call_site());

                                    if let Some(_) = return_type.optional {
                                        let v = quote! {
                                            #[serde(skip_serializing_if="Option::is_none")]
                                            pub #name: Option<Vec<#ref_type>>,
                                        };
                                        command_object.push(v);
                                    } else {
                                        let v = quote! {
                                            pub #name: Vec<#ref_type>,
                                        };
                                        command_object.push(v);
                                    }
                                }
                            } else {
                                let type_type: Option<Ident> = items.items_type.unwrap().into();

                                if let Some(typ) = type_type {
                                    if let Some(_) = return_type.optional {
                                        let v = quote! {
                                            #[serde(skip_serializing_if="Option::is_none")]
                                            pub #name: Option<Vec<#typ>>,
                                        };

                                        command_object.push(v);
                                    } else {
                                        let v = quote! {
                                            pub #name: Vec<#typ>,
                                        };

                                        command_object.push(v);
                                    }
                                }
                            }
                        }
                        TypeEnum::String => {
                            if let Some(enum_vec) = &return_type.parameter_enum {
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

                                let mut enum_name = name.to_string().to_case(Case::Pascal);
                                // let mut sp = p_name.to_string();
                                // sp.first_uppercase();
                                // enum_name.push_str(&sp);
                                enum_name.push_str("Option");
                                let enum_name = Ident::new(&enum_name, Span::call_site());

                                let vec: Vec<&String> = enum_vec
                                    .iter()
                                    .filter(|v| {
                                        let c = v.chars().next().unwrap();
                                        if c.is_uppercase() {
                                            true
                                        } else if v.contains("-") {
                                            true
                                        } else {
                                            false
                                        }
                                    })
                                    .collect();

                                let mut rename = quote! {
                                    #[serde(rename_all = "camelCase")]
                                };
                                if vec.len() > 0 {
                                    let v = vec[0];

                                    let c = v.chars().next().unwrap();

                                    if c.is_uppercase() {
                                        rename = quote! {
                                        #[serde(rename_all = "PascalCase")]
                                        }
                                    } else if v.contains("-") {
                                        rename = quote! {
                                        #[serde(rename_all = "kebab-case")]
                                        }
                                    }
                                }

                                let typ_enum = quote! {
                                    #[derive(Deserialize,Serialize, Debug,Clone,PartialEq)]
                                    #rename
                                    pub enum #enum_name {
                                        #(#enum_tokens)*
                                    }
                                };

                                enums.push(typ_enum);

                                if let Some(_) = return_type.optional {
                                    let v = quote! {
                                        #[serde(skip_serializing_if="Option::is_none")]
                                        pub #name: Option<#enum_name>,
                                    };
                                    command_object.push(v);
                                } else {
                                    let v = quote! {
                                        pub #name: #enum_name,
                                    };
                                    command_object.push(v);
                                }
                            } else {
                                if let Some(_) = return_type.optional {
                                    let v = quote! {
                                        #[serde(skip_serializing_if="Option::is_none")]
                                        pub #name: Option<String>,
                                    };

                                    command_object.push(v);
                                } else {
                                    let v = quote! {
                                        pub #name: String,
                                    };

                                    command_object.push(v);
                                }
                            }
                        }
                        _ => {
                            let mut type_type: Option<Ident> = param_type.into();

                            if let Some(typ) = type_type {
                                if let Some(_) = return_type.optional {
                                    let v = quote! {
                                        #[serde(skip_serializing_if="Option::is_none")]
                                        pub #name: Option<#typ>,
                                    };

                                    command_object.push(v);
                                } else {
                                    let v = quote! {
                                        pub #name: #typ,
                                    };

                                    command_object.push(v);
                                }
                            }
                        }
                    }
                } else {
                    let p_ref = &return_type.parameter_ref.clone().unwrap();

                    let ret_type =
                        Ident::new(&return_type.name.to_case(Case::Snake), Span::call_site());

                    if p_ref.contains(".") {
                        let dep = p_ref
                            .split(".")
                            .map(|v| Ident::new(v, Span::call_site()))
                            .collect::<Vec<Ident>>();

                        if let Some(_) = return_type.optional {
                            let v = quote! {
                                #[serde(skip_serializing_if="Option::is_none")]
                                pub #ret_type: Option<#(#dep)::*>,
                            };
                            command_object.push(v);
                        } else {
                            let v = quote! {
                                pub #ret_type: #(#dep)::*,
                            };
                            command_object.push(v);
                        }
                    } else {
                        let p_ref = Ident::new(&p_ref, Span::call_site());

                        if let Some(_) = return_type.optional {
                            let v = quote! {
                                #[serde(skip_serializing_if="Option::is_none")]
                                pub #ret_type: Option<#p_ref>,
                            };

                            command_object.push(v);
                        } else {
                            let v = quote! {
                                pub #ret_type: #p_ref,
                            };

                            command_object.push(v);
                        }
                    }
                }
            }
            command_objects.push(quote! {
                #[derive(Deserialize,Serialize, Debug,Clone,PartialEq)]
                #[serde(rename_all = "camelCase")]
                pub struct #name {
                    #(#command_object)*
                }
            });
        } else {
            command_objects.push(quote! {
                #[derive(Deserialize,Serialize, Debug,Clone,PartialEq)]
                #[serde(rename_all = "camelCase")]
                pub struct #name {}
            });
        }

        get_parameters(command, dependencies, parameter_objects, enums);
    }
}

pub fn get_parameters(
    command: &Command,
    dependencies: &mut Vec<TokenStream>,
    parameter_objects: &mut Vec<TokenStream>,
    enums: &mut Vec<TokenStream>,
) {
    let mut name = command.name.clone();
    name.first_uppercase();
    let name = Ident::new(&name, Span::call_site());

    if let Some(parameters) = command.parameters.as_deref() {
        let mut parameter_object: Vec<TokenStream> = Vec::new();
        for parameter in parameters {
            let p_name = Ident::new(
                &parameter
                    .name
                    .to_case(Case::Snake)
                    .replace("type", "Type")
                    .replace("override", "Override"),
                Span::call_site(),
            );

            if let Some(param_type) = parameter.parameter_type {
                match param_type {
                    TypeEnum::Array => {
                        let items = parameter.items.as_ref().unwrap();

                        if let Some(ref_type) = items.items_ref.clone() {
                            if ref_type.contains(".") {
                                let dep = ref_type
                                    .split(".")
                                    .map(|v| Ident::new(v, Span::call_site()))
                                    .collect::<Vec<Ident>>();

                                let v: Vec<&TokenStream> = dependencies
                                    .iter()
                                    .filter(|v| {
                                        let r = ref_type.split(".").collect::<Vec<&str>>()[0];
                                        v.to_string().contains(r)
                                    })
                                    .collect();

                                if v.len() <= 0 {
                                    let first_dep = &dep[0];
                                    dependencies.push(quote! {
                                        use super::#first_dep;
                                    });
                                }

                                if let Some(_) = parameter.optional {
                                    let v = quote! {
                                        #[serde(skip_serializing_if="Option::is_none")]
                                        pub #p_name: Option<#(#dep)::*>,
                                    };
                                    parameter_object.push(v);
                                } else {
                                    let v = quote! {
                                        pub #p_name: #(#dep)::*,
                                    };
                                    parameter_object.push(v);
                                }
                            } else {
                                let ref_type = Ident::new(&ref_type, Span::call_site());

                                if let Some(_) = parameter.optional {
                                    let v = quote! {
                                        #[serde(skip_serializing_if="Option::is_none")]
                                        pub #p_name: Option<Vec<#ref_type>>,
                                    };
                                    parameter_object.push(v);
                                } else {
                                    let v = quote! {
                                        pub #p_name: Vec<#ref_type>,
                                    };
                                    parameter_object.push(v);
                                }
                            }
                        } else {
                            let type_type: Option<Ident> = items.items_type.unwrap().into();

                            if let Some(typ) = type_type {
                                if let Some(_) = parameter.optional {
                                    let v = quote! {
                                        #[serde(skip_serializing_if="Option::is_none")]
                                        pub #p_name: Option<Vec<#typ>>,
                                    };

                                    parameter_object.push(v);
                                } else {
                                    let v = quote! {
                                        pub #p_name: Vec<#typ>,
                                    };

                                    parameter_object.push(v);
                                }
                            }
                        }
                    }
                    TypeEnum::String => {
                        if let Some(enum_vec) = &parameter.parameter_enum {
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

                            let mut enum_name = name.to_string();
                            let mut sp = p_name.to_string();
                            sp.first_uppercase();
                            enum_name.push_str(&sp);
                            enum_name.push_str("Option");
                            let enum_name = Ident::new(&enum_name, Span::call_site());
                            
                            let vec: Vec<&String> = enum_vec
                                    .iter()
                                    .filter(|v| {
                                        let c = v.chars().next().unwrap();
                                        if c.is_uppercase() {
                                            true
                                        } else if v.contains("-") {
                                            true
                                        } else {
                                            false
                                        }
                                    })
                                    .collect();

                                let mut rename = quote! {
                                    #[serde(rename_all = "camelCase")]
                                };
                                if vec.len() > 0 {
                                    let v = vec[0];

                                    let c = v.chars().next().unwrap();

                                    if c.is_uppercase() {
                                        rename = quote! {
                                        #[serde(rename_all = "PascalCase")]
                                        }
                                    } else if v.contains("-") {
                                        rename = quote! {
                                        #[serde(rename_all = "kebab-case")]
                                        }
                                    }
                                }

                            let typ_enum = quote! {
                                #[derive(Deserialize,Serialize, Debug,Clone,PartialEq)]
                                #rename
                                pub enum #enum_name {
                                    #(#enum_tokens)*
                                }
                            };

                            enums.push(typ_enum);

                            if let Some(_) = parameter.optional {
                                let v = quote! {
                                    #[serde(skip_serializing_if="Option::is_none")]
                                    pub #p_name: Option<#enum_name>,
                                };
                                parameter_object.push(v);
                            } else {
                                let v = quote! {
                                    pub #p_name: #enum_name,
                                };
                                parameter_object.push(v);
                            }
                        } else {
                            if let Some(_) = parameter.optional {
                                let v = quote! {
                                    #[serde(skip_serializing_if="Option::is_none")]
                                    pub #p_name: Option<String>,
                                };

                                parameter_object.push(v);
                            } else {
                                let v = quote! {
                                    pub #p_name: String,
                                };

                                parameter_object.push(v);
                            }
                        }
                    }
                    _ => {
                        let mut type_type: Option<Ident> = param_type.into();

                        let v = p_name.to_string();

                        if let Some(typ) = type_type {
                            if let Some(_) = parameter.optional {
                                let v = quote! {
                                    #[serde(skip_serializing_if="Option::is_none")]
                                    pub #p_name: Option<#typ>,
                                };

                                parameter_object.push(v);
                            } else {
                                let v = quote! {
                                    pub #p_name: #typ,
                                };

                                parameter_object.push(v);
                            }
                        }
                    }
                }
            } else {
                let p_ref = &parameter.parameter_ref.clone().unwrap();

                let ret_type = Ident::new(
                    &parameter.name.to_case(Case::Snake).replace("type", "Type"),
                    Span::call_site(),
                );

                if p_ref.contains(".") {
                    let dep = p_ref
                        .split(".")
                        .map(|v| Ident::new(v, Span::call_site()))
                        .collect::<Vec<Ident>>();

                    let v: Vec<&TokenStream> = dependencies
                        .iter()
                        .filter(|v| {
                            let r = p_ref.split(".").collect::<Vec<&str>>()[0];
                            v.to_string().contains(r)
                        })
                        .collect();

                    if v.len() <= 0 {
                        let first_dep = &dep[0];
                        dependencies.push(quote! {
                            use super::#first_dep;
                        });
                    }

                    if let Some(_) = parameter.optional {
                        let v = quote! {
                            #[serde(skip_serializing_if="Option::is_none")]
                            pub #ret_type: Option<#(#dep)::*>,
                        };
                        parameter_object.push(v);
                    } else {
                        let v = quote! {
                            pub #ret_type: #(#dep)::*,
                        };
                        parameter_object.push(v);
                    }
                } else {
                    let p_ref = Ident::new(&p_ref, Span::call_site());

                    if let Some(_) = parameter.optional {
                        let v = quote! {
                            #[serde(skip_serializing_if="Option::is_none")]
                            pub #ret_type: Option<#p_ref>,
                        };

                        parameter_object.push(v);
                    } else {
                        let v = quote! {
                            pub #ret_type: #p_ref,
                        };

                        parameter_object.push(v);
                    }
                }
            }
        }
        parameter_objects.push(quote! {
            #[derive(Deserialize,Serialize, Debug,Clone,PartialEq)]
            #[serde(rename_all = "camelCase")]
            pub struct #name {
                #(#parameter_object)*
            }
        });
    } else {
        parameter_objects.push(quote! {
            #[derive(Deserialize,Serialize, Debug,Clone,PartialEq)]
            #[serde(rename_all = "camelCase")]
            pub struct #name {}
        });
    }
}

pub fn get_events(
    event: Event,
    event_objects: &mut Vec<TokenStream>,
    enums: &mut Vec<TokenStream>,
) {
    let mut name = event.name.clone();
    name.first_uppercase();
    name.push_str("Event");
    let name = Ident::new(&name, Span::call_site());
    if let Some(parameters) = event.parameters {
        let mut event_object = Vec::new();
        for parameter in parameters {
            let p_name = Ident::new(
                &parameter
                    .name
                    .to_case(Case::Snake)
                    .replace("type", "Type")
                    .replace("override", "Override"),
                Span::call_site(),
            );

            if let Some(param_type) = parameter.parameter_type {
                match param_type {
                    TypeEnum::Array => {
                        let items = parameter.items.as_ref().unwrap();

                        if let Some(ref_type) = items.items_ref.clone() {
                            if ref_type.contains(".") {
                                let dep = ref_type
                                    .split(".")
                                    .map(|v| Ident::new(v, Span::call_site()))
                                    .collect::<Vec<Ident>>();

                                if let Some(_) = parameter.optional {
                                    let v = quote! {
                                        #[serde(skip_serializing_if="Option::is_none")]
                                        pub #p_name: Option<super::super::#(#dep)::*>,
                                    };
                                    event_object.push(v);
                                } else {
                                    let v = quote! {
                                        pub #p_name: super::super::#(#dep)::*,
                                    };
                                    event_object.push(v);
                                }
                            } else {
                                let ref_type = Ident::new(&ref_type, Span::call_site());

                                if let Some(_) = parameter.optional {
                                    let v = quote! {
                                        #[serde(skip_serializing_if="Option::is_none")]
                                        pub #p_name: Option<Vec<super::#ref_type>>,
                                    };
                                    event_object.push(v);
                                } else {
                                    let v = quote! {
                                        pub #p_name: Vec<super::#ref_type>,
                                    };
                                    event_object.push(v);
                                }
                            }
                        } else {
                            let type_type: Option<Ident> = items.items_type.unwrap().into();

                            if let Some(typ) = type_type {
                                if let Some(_) = parameter.optional {
                                    let v = quote! {
                                        #[serde(skip_serializing_if="Option::is_none")]
                                        pub #p_name: Option<Vec<#typ>>,
                                    };

                                    event_object.push(v);
                                } else {
                                    let v = quote! {
                                        pub #p_name: Vec<#typ>,
                                    };

                                    event_object.push(v);
                                }
                            }
                        }
                    }
                    TypeEnum::String => {
                        if let Some(enum_vec) = &parameter.parameter_enum {
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

                            let mut enum_name = name.to_string();
                            let mut sp = p_name.to_string();
                            sp.first_uppercase();
                            enum_name.push_str(&sp);
                            enum_name.push_str("Option");
                            let enum_name = Ident::new(&enum_name, Span::call_site());

                            let vec: Vec<&String> = enum_vec
                            .iter()
                            .filter(|v| {
                                let c = v.chars().next().unwrap();
                                if c.is_uppercase() {
                                    true
                                } else if v.contains("-") {
                                    true
                                } else {
                                    false
                                }
                            })
                            .collect();

                        let mut rename = quote! {
                            #[serde(rename_all = "camelCase")]
                        };
                        if vec.len() > 0 {
                            let v = vec[0];

                            let c = v.chars().next().unwrap();

                            if c.is_uppercase() {
                                rename = quote! {
                                #[serde(rename_all = "PascalCase")]
                                }
                            } else if v.contains("-") {
                                rename = quote! {
                                #[serde(rename_all = "kebab-case")]
                                }
                            }
                        }

                            let typ_enum = quote! {
                                #[derive(Deserialize,Serialize, Debug,Clone,PartialEq)]
                                #rename
                                pub enum #enum_name {
                                    #(#enum_tokens)*
                                }
                            };

                            enums.push(typ_enum);

                            if let Some(_) = parameter.optional {
                                let v = quote! {
                                    #[serde(skip_serializing_if="Option::is_none")]
                                    pub #p_name: Option<super::#enum_name>,
                                };
                                event_object.push(v);
                            } else {
                                let v = quote! {
                                    pub #p_name: super::#enum_name,
                                };
                                event_object.push(v);
                            }
                        } else {
                            if let Some(_) = parameter.optional {
                                let v = quote! {
                                    #[serde(skip_serializing_if="Option::is_none")]
                                    pub #p_name: Option<String>,
                                };

                                event_object.push(v);
                            } else {
                                let v = quote! {
                                    pub #p_name: String,
                                };

                                event_object.push(v);
                            }
                        }
                    }
                    _ => {
                        let mut type_type: Option<Ident> = param_type.into();

                        let v = p_name.to_string();

                        if let Some(typ) = type_type {
                            if let Some(_) = parameter.optional {
                                let v = quote! {
                                    #[serde(skip_serializing_if="Option::is_none")]
                                    pub #p_name: Option<#typ>,
                                };

                                event_object.push(v);
                            } else {
                                let v = quote! {
                                    pub #p_name: #typ,
                                };

                                event_object.push(v);
                            }
                        }
                    }
                }
            } else {
                let p_ref = &parameter.parameter_ref.clone().unwrap();

                let ret_type = Ident::new(
                    &parameter.name.to_case(Case::Snake).replace("type", "Type"),
                    Span::call_site(),
                );

                if p_ref.contains(".") {
                    let dep = p_ref
                        .split(".")
                        .map(|v| Ident::new(v, Span::call_site()))
                        .collect::<Vec<Ident>>();

                    if let Some(_) = parameter.optional {
                        let v = quote! {
                            #[serde(skip_serializing_if="Option::is_none")]
                            pub #ret_type: Option<super::super::#(#dep)::*>,
                        };
                        event_object.push(v);
                    } else {
                        let v = quote! {
                            pub #ret_type: super::super::#(#dep)::*,
                        };
                        event_object.push(v);
                    }
                } else {
                    let p_ref = Ident::new(&p_ref, Span::call_site());

                    if let Some(_) = parameter.optional {
                        let v = quote! {
                            #[serde(skip_serializing_if="Option::is_none")]
                            pub #ret_type: Option<super::#p_ref>,
                        };

                        event_object.push(v);
                    } else {
                        let v = quote! {
                            pub #ret_type: super::#p_ref,
                        };

                        event_object.push(v);
                    }
                }
            }
        }
        let mut param_name = name.to_string();
        param_name.push_str("Params");

        let param_ident = Ident::new(&param_name, Span::call_site());
        event_objects.push(quote! {
            #[derive(Deserialize,Serialize, Debug,Clone,PartialEq)]
            pub struct #name {
                pub params: #param_ident
            }

            #[derive(Deserialize,Serialize, Debug, Clone, PartialEq)]
            #[serde(rename_all = "camelCase")]
            pub struct #param_ident {
                #(#event_object)*
            }
        });
    } else {
        event_objects.push(quote! {
            #[derive(Deserialize,Serialize, Debug,Clone,PartialEq)]
            #[serde(rename_all = "camelCase")]
            pub struct #name {}
        });
    }
}

pub fn compile_cdp_json(file_name: &str) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let url = format!(
        "https://raw.githubusercontent.com/ChromeDevTools/devtools-protocol/master/json/{}",
        file_name
    );

    let json = reqwest::blocking::get(url)
        .expect("incorrect file name")
        .text()
        .unwrap();

    let protocol: Protocol = serde_json::from_str(&json).unwrap();

    let mut mods = Vec::new();
    let mut event_parts = Vec::new();

    for dom in protocol.domains {
        let mut types = Vec::new();
        let mut enums = Vec::new();
        let mut objects = Vec::new();
        let mut dependencies = Vec::new();

        let mut command_objects = Vec::new();

        let mut parameter_objects = Vec::new();

        let mut event_objects = Vec::new();

        let mut method_impls = Vec::new();

        if let Some(deps) = &dom.dependencies {
            for dep in deps
                .iter()
                .map(|v| Ident::new(&v.trim(), Span::call_site()))
                .collect::<Vec<Ident>>()
            {
                dependencies.push(quote! {
                    use super::#dep;
                });
            }
        }

        if let Some(type_elements) = dom.types.as_deref() {
            for type_element in type_elements {
                get_types(
                    type_element.type_type,
                    PropertyType::Element(type_element),
                    Some(type_element.clone()),
                    &mut types,
                    &mut enums,
                    &mut objects,
                    &mut Vec::new(),
                    &mut dependencies,
                    None,
                );
            }
        }

        get_commands(
            &dom.commands,
            &mut dependencies,
            &mut command_objects,
            &mut parameter_objects,
            &mut enums,
        );

        for command in &dom.commands {
            let mut cmd_name = command.name.clone();
            let mut method_name = String::from(dom.domain.clone());
            method_name.push_str(&format!(".{}", cmd_name));
            cmd_name.first_uppercase();

            let method_ident = Ident::new(&cmd_name, Span::call_site());

            let mut method_return_obj = cmd_name.clone();

            method_return_obj.push_str("ReturnObject");

            let method_return_obj = Ident::new(&method_return_obj, Span::call_site());

            let v = quote! {
                impl Method for #method_ident {
                    const NAME: &'static str = #method_name;
                    type ReturnObject = #method_return_obj;
                }
            };

            method_impls.push(v);
        }

        if let Some(events) = dom.events {
            for event in events {
                let event_name = event.name.clone();

                get_events(event, &mut event_objects, &mut enums);

                let mut domain_event = dom.domain.clone();

                domain_event.push_str(&format!(".{}", event_name));

                let domain_ident = Ident::new(&dom.domain.clone(), Span::call_site());

                let mut name = event_name.clone();

                name.first_uppercase();

                let mut enum_name = String::new();

                if !name.contains(&dom.domain) {
                    enum_name.push_str(&dom.domain.clone());
                }

                enum_name.push_str(&name);

                let enum_name = Ident::new(&enum_name, Span::call_site());

                name.push_str("Event");

                let name = Ident::new(&name, Span::call_site());

                let v = quote! {
                    #[serde(rename = #domain_event)]
                    #enum_name(super::#domain_ident::events::#name),
                };

                event_parts.push(v);
            }
        }

        let domain_ident = Ident::new(&dom.domain, Span::call_site());

        mods.push(quote! {
            pub mod #domain_ident {

                use serde::{Deserialize, Serialize};
                use serde_json::Value as Json;
                use super::types::*;

                #(#dependencies)*

                #(#types)*

                #(#enums)*

                #(#objects)*

                #(#parameter_objects)*

                #(#command_objects)*


                #(#method_impls)*

                pub mod events {
                    use serde::{Deserialize, Serialize};
                    use super::super::types::*;

                    #(#event_objects)*
                }

            }
        });
    }

    (mods, event_parts)
}
