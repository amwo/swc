#[macro_use]
extern crate pmutil;
extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
extern crate swc_macros_common;
extern crate syn;

use pmutil::{Quote, ToTokensExt};
use proc_macro::TokenStream;
use swc_macros_common::prelude::*;
use syn::{fold::Fold, *};

mod fold;

#[proc_macro_attribute]
pub fn emitter(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item: ImplItemMethod = syn::parse(item).expect("failed to parse input as an item");
    let item = fold::InjectSelf { parser: None }.fold_impl_item_method(item);
    let item = expand(item);

    print("emitter", item.dump())
}

fn expand(i: ImplItemMethod) -> ImplItemMethod {
    let mtd_name = i.sig.ident.clone();
    assert!(
        format!("{}", i.sig.ident).starts_with("emit_"),
        "#[emitter] methods should start with `emit_`"
    );
    let block = {
        let node_type = {
            i.sig
                .decl
                .inputs
                .clone()
                .into_iter()
                .skip(1)
                .next()
                .and_then(|arg| match arg {
                    FnArg::Ignored(ty) | FnArg::Captured(ArgCaptured { ty, .. }) => Some(ty),
                    _ => None,
                })
                .map(|ty| {
                    // &Ident -> Ident
                    match ty {
                        Type::Reference(TypeReference { elem, .. }) => *elem,
                        _ => panic!(
                            "Type of node parameter should be reference but got {}",
                            ty.dump()
                        ),
                    }
                })
                .expect(
                    "#[emitter] methods should have signature of 
fn (&mut self, node: Node) -> Result; 
    ",
                )
        };

        Quote::new_call_site()
            .quote_with(smart_quote!(
                Vars {
                    block: &i.block,
                    NodeType: &node_type,
                    mtd_name,
                },
                {
                    {
                        const _FOO: () = {
                            impl ::Node for NodeType {
                                fn emit_with(&self, e: &mut ::Emitter) -> Result {
                                    e.mtd_name(self)
                                }
                            }
                            ()
                        };

                        {
                            block
                        };

                        // Emitter methods return Result<_, _>
                        // We inject this to avoid writing Ok(()) every time.
                        #[allow(unreachable_code)]
                        {
                            return Ok(());
                        }
                    }
                }
            ))
            .parse()
    };

    ImplItemMethod { block, ..i }
}
