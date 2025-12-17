// SIMD optimizations for vector operations
// Inspired by Redis (prefetch patterns, scalar fallbacks) and Qdrant (SIMD hierarchy)
// Uses platform-specific SIMD intrinsics for maximum performance

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

// Minimum dimension sizes for SIMD (qdrant pattern)
#[cfg(target_arch = "x86_64")]
const MIN_DIM_SIZE_AVX: usize = 32;

#[cfg(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64"))]
const MIN_DIM_SIZE_SIMD: usize = 16;

/// SIMD-optimized dot product for cosine similarity
/// Vectors should be normalized for cosine similarity
/// Uses optimized scalar code with better pipelining (like Redis fallback)
#[inline]
pub fn dot_product_simd(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    // Try platform-specific SIMD if available (qdrant hierarchy pattern)
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") 
            && is_x86_feature_detected!("fma") 
            && a.len() >= MIN_DIM_SIZE_AVX 
        {
            return unsafe { dot_product_avx2(a, b) };
        }
    }
    
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("sse") && a.len() >= MIN_DIM_SIZE_SIMD {
            return unsafe { dot_product_sse(a, b) };
        }
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") && a.len() >= MIN_DIM_SIZE_SIMD {
            return unsafe { dot_product_neon(a, b) };
        }
    }
    
    // Optimized scalar fallback (like Redis's scalar implementation)
    // Uses two accumulators for better pipelining
    dot_product_scalar(a, b)
}

/// AVX2-optimized dot product (16 floats at a time)
/// Inspired by Redis's vectors_distance_float_avx2
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2", enable = "fma")]
#[inline]
unsafe fn dot_product_avx2(a: &[f32], b: &[f32]) -> f32 {
    let dim = a.len();
    let mut i = 0;
    
    let mut sum1 = _mm256_setzero_ps();
    let mut sum2 = _mm256_setzero_ps();
    
    // Process 16 floats at a time with two AVX2 registers
    while i + 15 < dim {
        let vx1 = _mm256_loadu_ps(a.as_ptr().add(i));
        let vy1 = _mm256_loadu_ps(b.as_ptr().add(i));
        let vx2 = _mm256_loadu_ps(a.as_ptr().add(i + 8));
        let vy2 = _mm256_loadu_ps(b.as_ptr().add(i + 8));
        
        sum1 = _mm256_fmadd_ps(vx1, vy1, sum1);
        sum2 = _mm256_fmadd_ps(vx2, vy2, sum2);
        
        i += 16;
    }
    
    // Combine the two sums
    let combined = _mm256_add_ps(sum1, sum2);
    
    // Horizontal sum of the 8 elements
    let sum_high = _mm256_extractf128_ps(combined, 1);
    let sum_low = _mm256_castps256_ps128(combined);
    let mut sum_128 = _mm_add_ps(sum_high, sum_low);
    
    sum_128 = _mm_hadd_ps(sum_128, sum_128);
    sum_128 = _mm_hadd_ps(sum_128, sum_128);
    
    let mut dot = _mm_cvtss_f32(sum_128);
    
    // Handle remaining elements
    while i < dim {
        dot += a[i] * b[i];
        i += 1;
    }
    
    dot
}

/// SSE-optimized dot product (qdrant compatibility pattern)
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse")]
#[inline]
unsafe fn dot_product_sse(a: &[f32], b: &[f32]) -> f32 {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;
    
    let dim = a.len();
    let mut i = 0;
    let mut sum = _mm_setzero_ps();
    
    // Process 4 floats at a time
    while i + 3 < dim {
        let va = _mm_loadu_ps(a.as_ptr().add(i));
        let vb = _mm_loadu_ps(b.as_ptr().add(i));
        sum = _mm_add_ps(sum, _mm_mul_ps(va, vb));
        i += 4;
    }
    
    // Horizontal sum
    let shuf = _mm_shuffle_ps(sum, sum, 0b10_11_00_01);
    sum = _mm_add_ps(sum, shuf);
    let shuf = _mm_movehl_ps(sum, sum);
    sum = _mm_add_ss(sum, shuf);
    
    let mut dot = _mm_cvtss_f32(sum);
    
    // Handle remaining elements
    while i < dim {
        dot += a[i] * b[i];
        i += 1;
    }
    
    dot
}

/// NEON-optimized dot product for ARM/Apple Silicon
/// Uses 8-wide processing with two NEON registers for better throughput
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
#[inline]
unsafe fn dot_product_neon(a: &[f32], b: &[f32]) -> f32 {
    let dim = a.len();
    let mut i = 0;
    
    // Use two accumulators for better instruction pipelining
    let mut sum1 = vdupq_n_f32(0.0);
    let mut sum2 = vdupq_n_f32(0.0);
    
    // Process 8 floats at a time with two NEON registers
    while i + 7 < dim {
        let va1 = vld1q_f32(a.as_ptr().add(i));
        let vb1 = vld1q_f32(b.as_ptr().add(i));
        let va2 = vld1q_f32(a.as_ptr().add(i + 4));
        let vb2 = vld1q_f32(b.as_ptr().add(i + 4));
        
        sum1 = vfmaq_f32(sum1, va1, vb1);
        sum2 = vfmaq_f32(sum2, va2, vb2);
        
        i += 8;
    }
    
    // Process remaining 4 floats
    while i + 3 < dim {
        let va = vld1q_f32(a.as_ptr().add(i));
        let vb = vld1q_f32(b.as_ptr().add(i));
        sum1 = vfmaq_f32(sum1, va, vb);
        i += 4;
    }
    
    // Combine accumulators and horizontal sum
    let combined = vaddq_f32(sum1, sum2);
    let mut dot = vaddvq_f32(combined);
    
    // Handle remaining elements
    while i < dim {
        dot += a[i] * b[i];
        i += 1;
    }
    
    dot
}

/// Scalar fallback (two accumulators for better pipelining - Redis pattern)
#[inline]
fn dot_product_scalar(a: &[f32], b: &[f32]) -> f32 {
    let mut dot0 = 0.0f32;
    let mut dot1 = 0.0f32;
    
    // Process 8 elements at a time with two accumulators
    let chunks = a.chunks_exact(8);
    let remainder = chunks.remainder();
    let b_chunks = b.chunks_exact(8);
    
    for (a_chunk, b_chunk) in chunks.zip(b_chunks) {
        dot0 += a_chunk[0] * b_chunk[0] +
                a_chunk[1] * b_chunk[1] +
                a_chunk[2] * b_chunk[2] +
                a_chunk[3] * b_chunk[3];
        
        dot1 += a_chunk[4] * b_chunk[4] +
                a_chunk[5] * b_chunk[5] +
                a_chunk[6] * b_chunk[6] +
                a_chunk[7] * b_chunk[7];
    }
    
    // Handle remainder
    for i in (a.len() - remainder.len())..a.len() {
        dot0 += a[i] * b[i];
    }
    
    dot0 + dot1
}


/// SIMD-optimized L2 distance (Euclidean)
#[inline]
pub fn l2_distance_simd(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::INFINITY;
    }
    
    // Try platform-specific SIMD if available (qdrant hierarchy pattern)
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") 
            && is_x86_feature_detected!("fma") 
            && a.len() >= MIN_DIM_SIZE_AVX 
        {
            return unsafe { l2_distance_avx2(a, b) };
        }
    }
    
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("sse") && a.len() >= MIN_DIM_SIZE_SIMD {
            return unsafe { l2_distance_sse(a, b) };
        }
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") && a.len() >= MIN_DIM_SIZE_SIMD {
            return unsafe { l2_distance_neon(a, b) };
        }
    }
    
    l2_distance_scalar(a, b)
}

/// AVX2-optimized L2 distance
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2", enable = "fma")]
#[inline]
unsafe fn l2_distance_avx2(a: &[f32], b: &[f32]) -> f32 {
    let dim = a.len();
    let mut i = 0;
    
    let mut sum1 = _mm256_setzero_ps();
    let mut sum2 = _mm256_setzero_ps();
    
    // Process 16 floats at a time with two AVX2 registers
    while i + 15 < dim {
        let va1 = _mm256_loadu_ps(a.as_ptr().add(i));
        let vb1 = _mm256_loadu_ps(b.as_ptr().add(i));
        let va2 = _mm256_loadu_ps(a.as_ptr().add(i + 8));
        let vb2 = _mm256_loadu_ps(b.as_ptr().add(i + 8));
        
        let diff1 = _mm256_sub_ps(va1, vb1);
        let diff2 = _mm256_sub_ps(va2, vb2);
        
        sum1 = _mm256_fmadd_ps(diff1, diff1, sum1);
        sum2 = _mm256_fmadd_ps(diff2, diff2, sum2);
        
        i += 16;
    }
    
    // Combine the two sums
    let combined = _mm256_add_ps(sum1, sum2);
    
    // Horizontal sum of the 8 elements
    let sum_high = _mm256_extractf128_ps(combined, 1);
    let sum_low = _mm256_castps256_ps128(combined);
    let mut sum_128 = _mm_add_ps(sum_high, sum_low);
    
    sum_128 = _mm_hadd_ps(sum_128, sum_128);
    sum_128 = _mm_hadd_ps(sum_128, sum_128);
    
    let mut sum_sq = _mm_cvtss_f32(sum_128);
    
    // Handle remaining elements
    while i < dim {
        let diff = a[i] - b[i];
        sum_sq += diff * diff;
        i += 1;
    }
    
    sum_sq.sqrt()
}

/// SSE-optimized L2 distance
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse")]
#[inline]
unsafe fn l2_distance_sse(a: &[f32], b: &[f32]) -> f32 {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;
    
    let dim = a.len();
    let mut i = 0;
    let mut sum = _mm_setzero_ps();
    
    // Process 4 floats at a time
    while i + 3 < dim {
        let va = _mm_loadu_ps(a.as_ptr().add(i));
        let vb = _mm_loadu_ps(b.as_ptr().add(i));
        let diff = _mm_sub_ps(va, vb);
        sum = _mm_add_ps(sum, _mm_mul_ps(diff, diff));
        i += 4;
    }
    
    // Horizontal sum
    let shuf = _mm_shuffle_ps(sum, sum, 0b10_11_00_01);
    sum = _mm_add_ps(sum, shuf);
    let shuf = _mm_movehl_ps(sum, sum);
    sum = _mm_add_ss(sum, shuf);
    
    let mut sum_sq = _mm_cvtss_f32(sum);
    
    // Handle remaining elements
    while i < dim {
        let diff = a[i] - b[i];
        sum_sq += diff * diff;
        i += 1;
    }
    
    sum_sq.sqrt()
}

/// NEON-optimized L2 distance for ARM/Apple Silicon
/// Uses 8-wide processing with two NEON registers for better throughput
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
#[inline]
unsafe fn l2_distance_neon(a: &[f32], b: &[f32]) -> f32 {
    let dim = a.len();
    let mut i = 0;
    
    // Use two accumulators for better instruction pipelining
    let mut sum1 = vdupq_n_f32(0.0);
    let mut sum2 = vdupq_n_f32(0.0);
    
    // Process 8 floats at a time with two NEON registers
    while i + 7 < dim {
        let va1 = vld1q_f32(a.as_ptr().add(i));
        let vb1 = vld1q_f32(b.as_ptr().add(i));
        let va2 = vld1q_f32(a.as_ptr().add(i + 4));
        let vb2 = vld1q_f32(b.as_ptr().add(i + 4));
        
        let diff1 = vsubq_f32(va1, vb1);
        let diff2 = vsubq_f32(va2, vb2);
        
        sum1 = vfmaq_f32(sum1, diff1, diff1);
        sum2 = vfmaq_f32(sum2, diff2, diff2);
        
        i += 8;
    }
    
    // Process remaining 4 floats
    while i + 3 < dim {
        let va = vld1q_f32(a.as_ptr().add(i));
        let vb = vld1q_f32(b.as_ptr().add(i));
        let diff = vsubq_f32(va, vb);
        sum1 = vfmaq_f32(sum1, diff, diff);
        i += 4;
    }
    
    // Combine accumulators and horizontal sum
    let combined = vaddq_f32(sum1, sum2);
    let mut sum_sq = vaddvq_f32(combined);
    
    // Handle remaining elements
    while i < dim {
        let diff = a[i] - b[i];
        sum_sq += diff * diff;
        i += 1;
    }
    
    sum_sq.sqrt()
}

/// Scalar L2 distance (two accumulators for better pipelining)
#[inline]
fn l2_distance_scalar(a: &[f32], b: &[f32]) -> f32 {
    let mut sum0 = 0.0f32;
    let mut sum1 = 0.0f32;
    
    // Process 4 elements at a time with two accumulators
    let chunks = a.chunks_exact(4);
    let remainder = chunks.remainder();
    let b_chunks = b.chunks_exact(4);
    
    for (a_chunk, b_chunk) in chunks.zip(b_chunks) {
        let d0 = a_chunk[0] - b_chunk[0];
        let d1 = a_chunk[1] - b_chunk[1];
        let d2 = a_chunk[2] - b_chunk[2];
        let d3 = a_chunk[3] - b_chunk[3];
        
        sum0 += d0 * d0 + d1 * d1;
        sum1 += d2 * d2 + d3 * d3;
    }
    
    // Handle remainder
    for i in (a.len() - remainder.len())..a.len() {
        let diff = a[i] - b[i];
        sum0 += diff * diff;
    }
    
    (sum0 + sum1).sqrt()
}

/// SIMD-optimized vector norm (squared length)
#[inline]
pub fn norm_squared_simd(v: &[f32]) -> f32 {
    dot_product_simd(v, v)
}

/// SIMD-optimized vector norm (length)
#[inline]
pub fn norm_simd(v: &[f32]) -> f32 {
    norm_squared_simd(v).sqrt()
}

