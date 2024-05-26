use crate::registry::PartialRegistry;
use crate::serialization::error::DeserializationError;
use crate::serialization::{DeserializeModel, SerializationFallback};
use duplicate::duplicate;
duplicate! {
    [
        ty;
        [ i8 ]; [ i16 ]; [ i32 ]; [ i64 ]; [ i128 ];
        [ u8 ]; [ u16 ]; [ u32 ]; [ u64 ]; [ u128 ];
        [ f32 ]; [ f64 ];
        [ bool ];
    ]
    impl<Registry: PartialRegistry> DeserializeModel<ty, Registry> for ty {
        #[inline(always)]
        fn deserialize(self, _registry: &mut Registry) -> Result<ty, DeserializationError<Registry>> {
            Ok(self)
        }
    }

    impl SerializationFallback for ty {
        type Fallback = ty;
    }
}
