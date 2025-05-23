/// Docs
///
/// More docs
pub struct CallbackNumPresInd {
    /// Docs
    ///
    /// More docs
    pub presentation: Presentation,
    pub screening: Screening,
}
#[automatically_derived]
impl ::core::fmt::Debug for CallbackNumPresInd {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field2_finish(
            f,
            "CallbackNumPresInd",
            "presentation",
            &self.presentation,
            "screening",
            &&self.screening,
        )
    }
}
impl ::rusmpp::encode::Length for CallbackNumPresInd {
    fn length(&self) -> usize {
        u8::from(*self).length()
    }
}
impl ::rusmpp::encode::Encode for CallbackNumPresInd {
    fn encode(&self, dst: &mut [u8]) -> usize {
        u8::from(*self).encode(dst)
    }
}
impl ::rusmpp::decode::Decode for CallbackNumPresInd {
    fn decode(src: &[u8]) -> Result<(Self, usize), ::rusmpp::decode::DecodeError> {
        u8::decode(src).map(|(this, size)| (Self::from(this), size))
    }
}
