// SPDX-License-Identifier: (Apache-2.0 OR MIT)

#[rustfmt::skip]
use std::arch::x86_64::{
    __m512i,
    _mm512_cmpgt_epi8_mask,
    _mm512_loadu_si512,
    _mm512_movm_epi8,
    _mm512_reduce_add_epi64,
    _mm512_sad_epu8,
    _mm512_set1_epi8,
    _mm512_setzero_si512,
    _mm512_sub_epi8,
};
use super::avx2::U8x32;

pub struct U8x64(__m512i);

impl U8x64 {
    pub const LEN: usize = 64;

    #[target_feature(enable = "avx512f")]
    #[inline]
    pub fn from_array(array: &[u8; 64]) -> Self {
        unsafe {
            let ptr = array.as_ptr().cast::<__m512i>();
            Self(_mm512_loadu_si512(ptr))
        }
    }

    #[target_feature(enable = "avx512f")]
    #[inline]
    pub fn splat0() -> Self {
        Self(_mm512_setzero_si512())
    }

    #[target_feature(enable = "avx512bw")]
    #[inline]
    pub fn sub(&self, other: &Self) -> Self {
        Self(_mm512_sub_epi8(self.0, other.0))
    }

    #[target_feature(enable = "avx512bw")]
    #[inline]
    pub fn mask_utf8_continuation_bytes(&self) -> Self {
        let v = _mm512_set1_epi8(-64);
        Self(_mm512_movm_epi8(_mm512_cmpgt_epi8_mask(v, self.0)))
    }

    #[target_feature(enable = "avx512bw")]
    #[inline]
    pub fn reduce_sum(&self) -> usize {
        let sums = _mm512_sad_epu8(self.0, _mm512_setzero_si512());
        _mm512_reduce_add_epi64(sums) as usize
    }
}

define_count_chars!(U8x64, U8x32, #[target_feature(enable = "avx512bw,avx512dq")]);
