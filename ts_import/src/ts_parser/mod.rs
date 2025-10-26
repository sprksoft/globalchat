use proc_macro2::TokenStream;
use unsynn::{unsynn, Any, Error, Ident, TokenIter, Transaction};

use crate::Visiblity;

mod ts_enum;
use ts_enum::TsEnum;

unsynn! {
    pub keyword Enum = "enum";
    pub keyword Export = "export";

    pub enum TsItem {
        Enum(TsEnum)
    }

    pub struct ExportedItem {
        export: Export,
        pub item: TsItem,
    }

    pub struct TsFile {
        exported_items: Any<ExportedItem>,
    }
}

impl TsFile {
    pub fn exported_items(&self) -> impl Iterator<Item = &ExportedItem> {
        self.exported_items.0.iter().map(|i| &i.value)
    }
}

impl TsItem {
    pub fn name(&self) -> &Ident {
        match self {
            Self::Enum(enumm) => &enumm.name,
        }
    }

    pub fn emit(&self, vis: &Visiblity) -> TokenStream {
        match self {
            Self::Enum(e) => e.emit(vis),
        }
    }
}

#[cfg(test)]
mod test {
    use unsynn::{Parse, ToTokens};

    use super::{ts_enum::TsEnumVariants, TsFile, TsItem};

    #[test]
    fn empty_lines() {
        let mut tokens = "export enum Test {Test = 5,\n\nTest2 = 2}".into_token_iter();
        let file = TsFile::parse(&mut tokens).unwrap();
        let TsItem::Enum(eenum) = &file.exported_items().next().unwrap().item;
        match eenum.variants() {
            TsEnumVariants::NumBacked(variants) => {
                let mut variants = variants.variant_names().map(|i| i.to_string());
                assert_eq!(variants.next(), Some("Test".to_string()));
                assert_eq!(variants.next(), Some("Test2".to_string()));
                assert_eq!(variants.next(), None);
            }
            _ => panic!("Wrong backing. on exported enum"),
        }

        let mut tokens =
            "export enum Test {Test = \"test\",\n\nTest2 = \"test\"}".into_token_iter();
        let file = TsFile::parse(&mut tokens).unwrap();
        let TsItem::Enum(eenum) = &file.exported_items().next().unwrap().item;
        match eenum.variants() {
            TsEnumVariants::StringBacked(variants) => {
                let mut variants = variants.variant_names().map(|i| i.to_string());
                assert_eq!(variants.next(), Some("Test".to_string()));
                assert_eq!(variants.next(), Some("Test2".to_string()));
                assert_eq!(variants.next(), None);
            }
            _ => panic!("Wrong backing. on exported enum"),
        }
    }
}
