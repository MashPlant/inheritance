use proc_macro::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, parse_macro_input, Visibility, Field, Result, Token, Ident};
use quote::quote;
use std::collections::HashMap;

#[proc_macro]
pub fn inheritance(input: TokenStream) -> TokenStream {
  let mut structs = parse_macro_input!(input as ItemStructs).0;
  let mut ident2id = HashMap::new();
  for (idx, s) in structs.iter().enumerate() {
    // if insert `&s.ident` into `ident2id`, it will borrow `structs`
    // which disallows following mutation of other fields
    if ident2id.insert(s.ident.clone(), idx).is_some() {
      panic!("duplicate struct name `{}`", s.ident);
    }
  }
  for idx in 0..structs.len() {
    if let Some(p) = &structs[idx].parent {
      if let Some(&p_idx) = ident2id.get(p) {
        structs[idx].parent_idx = Some(p_idx);
        structs[p_idx].children_idx.push(idx);
      } else { panic!("`{}`'s parent `{}` not found", structs[idx].ident, p); }
    }
  }
  // detect cyclic inheritance
  for idx in 0..structs.len() {
    let mut s = &mut structs[idx];
    loop {
      if s.visit_time == 0 {
        s.visit_time = idx + 1;
        if let Some(p) = s.parent_idx { s = &mut structs[p]; } else { break; }
      } else {
        if s.visit_time == idx + 1 { panic!("cyclic inheritance"); }
        break;
      }
    }
  }
  // since there is no cyclic inheritance, there must be at least a struct with 0 parent
  for idx in 0..structs.len() {
    fn dfs(structs: &mut Vec<ItemStruct>, idx: usize, discriminant: &mut usize) {
      for idx1 in 0..structs[idx].children_idx.len() {
        let ch_idx = structs[idx].children_idx[idx1];
        if structs[ch_idx].children_idx.is_empty() {
          structs[idx].concrete_children_idx.push(ch_idx);
          structs[ch_idx].discriminant = *discriminant;
          *discriminant += 1;
        } else {
          dfs(structs, ch_idx, discriminant);
          let mut tmp = structs[ch_idx].concrete_children_idx.clone();
          structs[idx].concrete_children_idx.append(&mut tmp);
        }
      }
    }
    if structs[idx].parent_idx.is_none() {
      let mut discriminant = 0;
      dfs(&mut structs, idx, &mut discriminant);
      if discriminant == 0 {
        panic!("no concrete subclass found for `{}`", structs[idx].ident);
      }
    }
  }
  let it = structs.iter().map(
    |ItemStruct { vis, ident, parent, fields, children_idx, concrete_children_idx, discriminant, .. }| {
      if children_idx.is_empty() {
        let p = if let Some(p) = parent { p } else { panic!("struct `{}` is isolated from other structs", ident) };
        let enum_ident = Ident::new(&(ident.to_string() + &p.to_string()), ident.span());
        let p_enum_ident = Ident::new(&("Generic".to_string() + &p.to_string()), ident.span());
        let fields_names = fields.iter().map(|f| &f.ident);
        quote! {
          #[repr(C)]
          #vis struct #ident {
            base: #p,
            #fields
          }

          impl ::core::ops::Deref for #ident {
            type Target = #p;
            fn deref(&self) -> &Self::Target { &self.base }
          }

          impl ::core::ops::DerefMut for #ident {
            fn deref_mut(&mut self) -> &mut Self::Target { &mut self.base }
          }

          #[repr(C, usize)]
          #vis enum #enum_ident { #ident(#ident) = #discriminant }

          impl #enum_ident {
            pub fn new(base: #p, #fields) -> #enum_ident {
              #enum_ident::#ident(#ident { base, #(#fields_names),* })
            }

            pub fn upcast(&self) -> &#p_enum_ident { unsafe { ::core::mem::transmute(self) } }
          }

          impl ::core::ops::Deref for #enum_ident {
            type Target = #ident;
            fn deref(&self) -> &Self::Target { match self { #enum_ident::#ident(x) => x } }
          }

          impl ::core::ops::DerefMut for #enum_ident {
            fn deref_mut(&mut self) -> &mut Self::Target { match self { #enum_ident::#ident(x) => x } }
          }
        }
      } else {
        let enum_ident = Ident::new(&("Generic".to_string() + &ident.to_string()), ident.span());
        let variants = concrete_children_idx.iter().map(|&idx| {
          let (ident, discriminant) = (&structs[idx].ident, structs[idx].discriminant);
          quote! { #ident(#ident) = #discriminant, }
        });
        quote! {
          #[repr(C)]
          #vis struct #ident { #fields }

          #[repr(C, usize)]
          #vis enum #enum_ident { #(#variants)* }

          impl ::core::ops::Deref for #enum_ident {
            type Target = #ident;
            fn deref(&self) -> &Self::Target {
              unsafe { &*((self as *const _ as *const u8).add(::core::mem::size_of::<usize>()) as *const _) }
            }
          }

          impl ::core::ops::DerefMut for #enum_ident {
            fn deref_mut(&mut self) -> &mut Self::Target {
              unsafe { &mut *((self as *mut _ as *mut u8).add(::core::mem::size_of::<usize>()) as *mut _) }
            }
          }
        }
      }
    });
  TokenStream::from(quote! { #(#it)* })
}

// todo: support Generics & other forms of struct later
#[derive(Debug)]
struct ItemStruct {
  vis: Visibility,
  ident: Ident,
  parent: Option<Ident>,
  fields: Punctuated<Field, Token![,]>,
  // below fields are synthetic
  parent_idx: Option<usize>,
  children_idx: Vec<usize>,
  visit_time: usize,
  concrete_children_idx: Vec<usize>,
  discriminant: usize,
}

impl Parse for ItemStruct {
  fn parse(input: ParseStream) -> Result<Self> {
    let content;
    let vis = input.parse()?;
    let _ = input.parse::<Token![struct]>()?;
    let ident = input.parse()?;
    let parent = if input.parse::<Token![:]>().is_ok() { Some(input.parse()?) } else { None };
    let _ = braced!(content in input);
    let fields = content.parse_terminated(Field::parse_named)?;
    Ok(ItemStruct { vis, ident, parent, fields, parent_idx: None, children_idx: Vec::new(), visit_time: 0, concrete_children_idx: Vec::new(), discriminant: 0 })
  }
}

struct ItemStructs(Vec<ItemStruct>);

impl Parse for ItemStructs {
  fn parse(input: ParseStream) -> Result<Self> {
    let mut ret = Vec::new();
    while !input.is_empty() { ret.push(input.parse()?); }
    Ok(ItemStructs(ret))
  }
}