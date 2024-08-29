
#[cfg(not(feature = "coef-all-one"))]
#[macro_export]
macro_rules! challenge_mul_circuit_field {
    ($challenge: expr, $circuit_field: expr) => {
        C::challenge_mul_circuit_field($challenge, $circuit_field)        
    }
}

#[cfg(feature = "coef-all-one")]
macro_rules! challenge_mul_circuit_field {
    ($challenge: expr, $circuit_field: expr) => {
        $challenge
    }
}


#[cfg(not(feature = "coef-all-one"))]
#[macro_export]
macro_rules! circuit_field_mul_simd_circuit_field {
    ($circuit_field: expr, $simd_circuit_field: expr) => {
        C::circuit_field_mul_simd_circuit_field($circuit_field, $simd_circuit_field)        
    }
}

#[cfg(feature = "coef-all-one")]
#[macro_export]
macro_rules! circuit_field_mul_simd_circuit_field {
    ($circuit_field: expr, $simd_circuit_field: expr) => {
        $simd_circuit_field
    }
}

