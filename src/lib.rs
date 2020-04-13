use proc_macro::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, parse_macro_input, Visibility, Field, Result, Token, Ident};
use quote::quote;
use std::collections::HashMap;

#[proc_macro]
pub fn inheritance(input: TokenStream) -> TokenStream {
  let mut ss = parse_macro_input!(input as Itemss).0;
  let mut ident2id = HashMap::new();
  for (idx, s) in ss.iter().enumerate() {
    // if insert `&s.ident` into `ident2id`, it will borrow `ss`
    // which disallows following mutation of other fields
    if ident2id.insert(s.ident.clone(), idx).is_some() {
      panic!("duplicate struct name `{}`", s.ident);
    }
  }
  for idx in 0..ss.len() {
    if let Some(p) = &ss[idx].parent {
      if let Some(&p_idx) = ident2id.get(p) {
        ss[idx].parent_idx = Some(p_idx);
        ss[p_idx].direct_ch_idx.push(idx);
      } else { panic!("`{}`'s parent `{}` not found", ss[idx].ident, p); }
    }
  }
  // detect cyclic inheritance
  for idx in 0..ss.len() {
    let mut s = &mut ss[idx];
    loop {
      if s.visit_time == 0 {
        s.visit_time = idx + 1;
        if let Some(p) = s.parent_idx { s = &mut ss[p]; } else { break; }
      } else {
        if s.visit_time == idx + 1 { panic!("cyclic inheritance"); }
        break;
      }
    }
  }
  // since there is no cyclic inheritance, there must be at least a struct with 0 parent
  for idx in 0..ss.len() {
    fn dfs(ss: &mut Vec<ItemStruct>, idx: usize, d: &mut usize) {
      for idx1 in 0..ss[idx].direct_ch_idx.len() {
        let ch_idx = ss[idx].direct_ch_idx[idx1];
        if ss[ch_idx].direct_ch_idx.is_empty() {
          ss[idx].concrete_ch_idx.push(ch_idx);
          ss[ch_idx].discriminant = *d;
          *d += 1;
        } else {
          dfs(ss, ch_idx, d);
          let mut tmp = ss[ch_idx].concrete_ch_idx.clone();
          ss[idx].concrete_ch_idx.append(&mut tmp);
          ss[idx].abstract_ch_idx.push(ch_idx);
          let mut tmp = ss[ch_idx].abstract_ch_idx.clone();
          ss[idx].abstract_ch_idx.append(&mut tmp);
        }
      }
    }
    if ss[idx].parent_idx.is_none() {
      let mut d = 0;
      dfs(&mut ss, idx, &mut d);
      if d == 0 {
        panic!("no concrete subclass found for `{}`", ss[idx].ident);
      }
    }
  }
  let it = ss.iter().map(
    |ItemStruct { vis, ident, parent, fields, direct_ch_idx, concrete_ch_idx, abstract_ch_idx, discriminant, .. }| {
      if direct_ch_idx.is_empty() {
        let p = if let Some(p) = parent { p } else { panic!("struct `{}` is isolated from other ss", ident) };
        let deref_impl = deref_impl(ident, p);
        let enum_ident = Ident::new(&(ident.to_string() + &p.to_string()), ident.span());
        let new_impl = new_impl(ident, p, fields, true);
        let p_enum_ident = Ident::new(&("Generic".to_string() + &p.to_string()), ident.span());
        quote! {
          #[repr(C)]
          #vis struct #ident {
            base: #p,
            #fields
          }
          #deref_impl

          #[repr(C, usize)]
          #vis enum #enum_ident { #ident(#ident) = #discriminant }
          impl #enum_ident {
            #new_impl
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
        let variants = concrete_ch_idx.iter().map(|&idx| {
          let (ident, d) = (&ss[idx].ident, ss[idx].discriminant);
          quote! { #ident(#ident) = #d, }
        });
        let base = parent.as_ref().map(|p| quote! { base: #p, });
        let new_impl = parent.as_ref().map(|p| new_impl(ident, p, fields, false));
        let deref_impl = parent.as_ref().map(|p| deref_impl(ident, p));
        let upcast = parent.as_ref().map(|p| {
          let p_enum_ident = Ident::new(&("Generic".to_string() + &p.to_string()), p.span());
          quote! { pub fn upcast(&self) -> &#p_enum_ident { unsafe { ::core::mem::transmute(self) } } }
        });
        let trait_ident = Ident::new(&(ident.to_string() + "Info"), ident.span());
        // the trait must be unsafe because one can implement it on other types without unsafe code easily
        // which makes `downcast` / `downcast_mut` able to cast into these types, which is unsafe
        let trait_def = if let Some(p) = parent {
          let p_trait_ident = Ident::new(&(p.to_string() + "Info"), p.span());
          quote! { pub unsafe trait #trait_ident: #p_trait_ident {} }
        } else {
          quote! { pub unsafe trait #trait_ident { fn classof(d: usize) -> bool; } }
        };
        let trait_impl = if parent.is_some() {
          let it = concrete_ch_idx.iter().chain(abstract_ch_idx.iter()).map(|&idx| ss[idx].enum_ident());
          quote! { #(unsafe impl #trait_ident for #it {})* }
        } else {
          let it1 = concrete_ch_idx.iter().map(|&idx| {
            let (ident, d) = (ss[idx].enum_ident(), ss[idx].discriminant);
            quote! { unsafe impl #trait_ident for #ident { fn classof(d: usize) -> bool { d == #d } } }
          });
          let it2 = abstract_ch_idx.iter().map(|&idx| {
            let ident = ss[idx].enum_ident();
            let ds = ss[idx].concrete_ch_idx.iter().map(|&idx| ss[idx].discriminant);
            quote! { unsafe impl #trait_ident for #ident { fn classof(d: usize) -> bool { #(d == #ds) ||* } } }
          });
          quote! { #(#it1)* #(#it2)* }
        };
        quote! {
          #[repr(C)]
          #vis struct #ident {
            #base
            #fields
          }
          impl #ident { #new_impl }
          #deref_impl

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
          impl #enum_ident {
            #upcast
            pub fn downcast<T: #trait_ident>(&self) -> Option<&T> {
              unsafe { if T::classof(*(self as *const _ as *const usize)) { Some(unsafe { ::core::mem::transmute(self) }) } else { None } }
            }
            pub fn downcast_mut<T: #trait_ident>(&mut self) -> Option<&mut T> {
              unsafe { if T::classof(*(self as *const _ as *const usize)) { Some(::core::mem::transmute(self)) } else { None } }
            }
          }
          #trait_def
          #trait_impl
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
  direct_ch_idx: Vec<usize>,
  visit_time: usize,
  concrete_ch_idx: Vec<usize>,
  abstract_ch_idx: Vec<usize>,
  discriminant: usize,
}

impl ItemStruct {
  fn enum_ident(&self) -> Ident {
    let name = self.ident.to_string();
    let name = if self.direct_ch_idx.is_empty() {
      name + &self.parent.as_ref().unwrap().to_string()
    } else { "Generic".to_string() + &name };
    Ident::new(&name, self.ident.span())
  }
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
    Ok(ItemStruct { vis, ident, parent, fields, parent_idx: None, direct_ch_idx: Vec::new(), visit_time: 0, concrete_ch_idx: Vec::new(), abstract_ch_idx: Vec::new(), discriminant: 0 })
  }
}

struct Itemss(Vec<ItemStruct>);

impl Parse for Itemss {
  fn parse(input: ParseStream) -> Result<Self> {
    let mut ret = Vec::new();
    while !input.is_empty() { ret.push(input.parse()?); }
    Ok(Itemss(ret))
  }
}

fn deref_impl(ident: &Ident, p: &Ident) -> proc_macro2::TokenStream {
  quote! {
    impl ::core::ops::Deref for #ident {
      type Target = #p;
      fn deref(&self) -> &Self::Target { &self.base }
    }
    impl ::core::ops::DerefMut for #ident {
      fn deref_mut(&mut self) -> &mut Self::Target { &mut self.base }
    }
  }
}

fn new_impl(ident: &Ident, p: &Ident, fields: &Punctuated<Field, Token![,]>, is_enum: bool) -> proc_macro2::TokenStream {
  let fields_names = fields.iter().map(|f| &f.ident);
  let field_name_tys = fields.iter().map(
    |Field { ident, ty, .. }| quote! { #ident: #ty });
  let mut ret = quote! { #ident { base, #(#fields_names),* } };
  if is_enum { ret = quote! { Self::#ident(#ret) }; }
  quote! { pub fn new(base: #p, #(#field_name_tys),*) -> Self { #ret } }
}