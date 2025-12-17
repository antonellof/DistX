// SIMD optimizations for vector operations (inspired by Redis)
// Uses platform-specific SIMD intrinsics for maximum performance

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// SIMD-optimized dot product for cosine similarity
/// Vectors should be normalized (like Redis does)
/// Uses optimized scalar code with better pipelining (like Redis fallback)
#[inline]
pub fn dot_product_simd(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    // Try platform-specific SIMD if available
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") && a.len() >= 16 {
            unsafe {
                return dot_product_avx2(a, b);
            }
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

/// Scalar fallback (two accumulators for better pipelining)
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
    
    // For now, use optimized scalar (can add AVX2 later)
    l2_distance_scalar(a, b)
}

/// Scalar L2 distance
#[inline]
fn l2_distance_scalar(a: &[f32], b: &[f32]) -> f32 {
    let sum_sq_diff: f32 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = x - y;
            diff * diff
        })
        .sum();
    
    sum_sq_diff.sqrt()
}

