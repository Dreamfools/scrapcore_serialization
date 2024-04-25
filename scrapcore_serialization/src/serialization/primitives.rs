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

#[cfg(feature = "glam")]
duplicate! {
    [
        ty;

        [ glam::f32::Vec2 ]; [ glam::f32::Vec3 ]; [ glam::f32::Vec4 ];
        [ glam::f64::DVec2 ]; [ glam::f64::DVec3 ]; [ glam::f64::DVec4 ];
        [ glam::i32::IVec2 ]; [ glam::i32::IVec3 ]; [ glam::i32::IVec4 ];
        [ glam::u32::UVec2 ]; [ glam::u32::UVec3 ]; [ glam::u32::UVec4 ];
        [ glam::i64::I64Vec2 ]; [ glam::i64::I64Vec3 ]; [ glam::i64::I64Vec4 ];
        [ glam::u64::U64Vec2 ]; [ glam::u64::U64Vec3 ]; [ glam::u64::U64Vec4 ];
        [ glam::bool::BVec2 ]; [ glam::bool::BVec3 ]; [ glam::bool::BVec4 ];
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
