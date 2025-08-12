//! Circuit serialization functionality for GPU processing

use circuit::Circuit;
use gkr_engine::{ExpanderDualVarChallenge, FieldEngine, Transcript};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};

#[derive(thiserror::Error, Debug)]
pub enum SerializationError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse field element: {0}")]
    FieldParse(String),
}

// Helper function to serialize a field element to a string representation
fn serialize_field_element<C: FieldEngine>(element: &C::CircuitField) -> String {
    // Get element size for auxiliary decision making
    let element_size = std::mem::size_of::<C::CircuitField>();

    // For BN254 case (element size is typically 32 bytes, i.e., 8 u32)
    if element_size == 32 {
        // Convert to u8 slice to inspect actual content
        let element_bytes = unsafe {
            std::slice::from_raw_parts(
                (element as *const C::CircuitField) as *const u8,
                element_size,
            )
        };

        // Convert bytes to hexadecimal representation
        let mut hex_str = String::with_capacity(2 + element_size * 2); // "0x" + two hexadecimal characters per byte
        hex_str.push_str("0x");

        // Append in little-endian order (from most significant byte)
        for i in (0..element_size).rev() {
            let byte = element_bytes[i];
            hex_str.push_str(&format!("{byte:02x}"));
        }

        // Check if it's zero value (all bytes are 0)
        let is_zero = element_bytes.iter().all(|&b| b == 0);
        if is_zero {
            return "0x0000000000000000000000000000000000000000000000000000000000000000"
                .to_string();
        }

        hex_str
    } else {
        // For other field types: m31ext3 and goldilocks
        let debug_str = format!("{element:?}");

        // Try to extract value from "FieldType { v: value }" format
        if let Some(v_pos) = debug_str.find("v: ") {
            let after_v = &debug_str[v_pos + 3..];
            if let Some(end_pos) = after_v.find(" }") {
                let value_str = &after_v[..end_pos];
                return value_str.trim().to_string();
            } else if let Some(end_pos) = after_v.find("}") {
                let value_str = &after_v[..end_pos];
                return value_str.trim().to_string();
            }
        }

        // Panic if parsing fails
        panic!("Failed to parse field element value from debug string: {debug_str}")
    }
}

// Helper function to serialize a SIMD field element to a string representation
fn serialize_simd_field_element<C: FieldEngine>(element: &C::SimdCircuitField) -> String
where
    C::SimdCircuitField: std::fmt::Debug,
{
    // Get element size for auxiliary decision making
    let element_size = std::mem::size_of::<C::SimdCircuitField>();

    // For BN254 case (element size is typically 32 bytes * SIMD width)
    if element_size >= 32 && element_size % 32 == 0 {
        // Convert to u8 slice to inspect actual content
        let element_bytes = unsafe {
            std::slice::from_raw_parts(
                (element as *const C::SimdCircuitField) as *const u8,
                element_size,
            )
        };

        // For SIMD, we might have multiple 32-byte field elements
        let num_elements = element_size / 32;
        if num_elements == 1 {
            // Single element case
            let mut hex_str = String::with_capacity(2 + 32 * 2);
            hex_str.push_str("0x");

            // Append in little-endian order (from most significant byte)
            for i in (0..32).rev() {
                let byte = element_bytes[i];
                hex_str.push_str(&format!("{byte:02x}"));
            }

            // Check if it's zero value (all bytes are 0)
            let is_zero = element_bytes[..32].iter().all(|&b| b == 0);
            if is_zero {
                return "0x0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string();
            }

            hex_str
        } else {
            // Multiple elements case - format as array
            let mut result = String::from("[");
            for elem_idx in 0..num_elements {
                if elem_idx > 0 {
                    result.push(',');
                }

                let start_byte = elem_idx * 32;
                let end_byte = start_byte + 32;

                let mut hex_str = String::with_capacity(2 + 32 * 2);
                hex_str.push_str("0x");

                // Append in little-endian order (from most significant byte)
                for i in (start_byte..end_byte).rev() {
                    let byte = element_bytes[i];
                    hex_str.push_str(&format!("{byte:02x}"));
                }

                // Check if it's zero value
                let is_zero = element_bytes[start_byte..end_byte].iter().all(|&b| b == 0);
                if is_zero {
                    result.push_str(
                        "0x0000000000000000000000000000000000000000000000000000000000000000",
                    );
                } else {
                    result.push_str(&hex_str);
                }
            }
            result.push(']');
            result
        }
    } else {
        // For other field types: m31ext3 and goldilocks
        let debug_str = format!("{element:?}");

        // Handle array format: [Type { v: val1 }, Type { v: val2 }, ...]
        if debug_str.contains('[') && debug_str.contains(']') {
            if let Some(start) = debug_str.find('[') {
                if let Some(end) = debug_str.rfind(']') {
                    let array_content = &debug_str[start + 1..end];

                    // Split by comma and extract values from each element
                    let elements: Vec<&str> = array_content.split(',').collect();
                    let mut extracted_values = Vec::new();

                    for element_str in elements {
                        let element_str = element_str.trim();
                        // Try to extract value from "FieldType { v: value }" format
                        if let Some(v_pos) = element_str.find("v: ") {
                            let after_v = &element_str[v_pos + 3..];
                            if let Some(end_pos) = after_v.find(" }") {
                                let value_str = &after_v[..end_pos];
                                extracted_values.push(value_str.trim().to_string());
                            } else if let Some(end_pos) = after_v.find("}") {
                                let value_str = &after_v[..end_pos];
                                extracted_values.push(value_str.trim().to_string());
                            } else {
                                // Fallback to original element string if parsing fails
                                extracted_values.push(element_str.to_string());
                            }
                        } else {
                            // Fallback to original element string if no "v: " found
                            extracted_values.push(element_str.to_string());
                        }
                    }

                    return format!("[{}]", extracted_values.join(","));
                }
            }
        } else {
            // Single element case - try to extract value from "FieldType { v: value }" format
            if let Some(v_pos) = debug_str.find("v: ") {
                let after_v = &debug_str[v_pos + 3..];
                if let Some(end_pos) = after_v.find(" }") {
                    let value_str = &after_v[..end_pos];
                    return value_str.trim().to_string();
                } else if let Some(end_pos) = after_v.find("}") {
                    let value_str = &after_v[..end_pos];
                    return value_str.trim().to_string();
                }
            }
        }

        // Panic if parsing fails
        panic!("Failed to parse SIMD field element value from debug string: {debug_str}")
    }
}

/// Serialize circuit to a file compatible with circuit.cuh
pub fn serialize_circuit_to_file<C: FieldEngine>(
    circuit: &Circuit<C>,
    filepath: &str,
) -> Result<(), SerializationError>
where
    C::CircuitField: std::fmt::Debug,
    C::SimdCircuitField: std::fmt::Debug,
{
    // Create data directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(filepath).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Use BufWriter to improve write efficiency
    let file = File::create(filepath)?;
    let mut writer = BufWriter::with_capacity(8 * 1024 * 1024, file); // 8MB buffer

    // Determine field type based on field size
    let field_type = match std::mem::size_of::<C::CircuitField>() {
        32 => "bn254",         // BN254 field
        4 => "m31ext3",        // m31ext3 field
        8 => "goldilocksext2", // goldilocks field
        _ => "unknown",        // unknown field
    };

    // Write header: TotalLayer [layer_count] [field_type]
    writeln!(writer, "TotalLayer {} {}", circuit.layers.len(), field_type)?;
    writer.flush()?; // Immediately flush header information

    // Count total items to serialize for progress calculation (gates + values)
    let total_gates: usize = circuit
        .layers
        .iter()
        .map(|layer| layer.add.len() + layer.mul.len())
        .sum();
    let total_values: usize = circuit
        .layers
        .iter()
        .map(|layer| layer.input_vals.len() + layer.output_vals.len())
        .sum();
    let total_items = total_gates + total_values;

    let mut items_processed = 0;
    let mut last_percent = 0;

    // Process each layer
    for (layer_idx, layer) in circuit.layers.iter().enumerate() {
        // Write layer header: Layer [num_gate_add] [num_gate_mul] [input_var_num] [output_var_num]
        // [input_vals_count] [output_vals_count]
        writeln!(
            writer,
            "Layer[{}] {} {} {} {} {} {}",
            layer_idx,
            layer.add.len(),
            layer.mul.len(),
            layer.input_var_num,
            layer.output_var_num,
            layer.input_vals.len(),
            layer.output_vals.len()
        )?;

        // Write input values
        if !layer.input_vals.is_empty() {
            writeln!(writer, "=====Input Values=====")?;
            for (idx, input_val) in layer.input_vals.iter().enumerate() {
                let val_str = serialize_simd_field_element::<C>(input_val);
                writeln!(writer, "InputVal[{idx}] {val_str}")?;
                items_processed += 1;
            }
        }

        // Write output values
        if !layer.output_vals.is_empty() {
            writeln!(writer, "=====Output Values=====")?;
            for (idx, output_val) in layer.output_vals.iter().enumerate() {
                let val_str = serialize_simd_field_element::<C>(output_val);
                writeln!(writer, "OutputVal[{idx}] {val_str}")?;
                items_processed += 1;
            }
        }

        // Write gates section marker
        if !layer.add.is_empty() || !layer.mul.is_empty() {
            writeln!(writer, "=====Gates=====")?;
        }

        // Every 10 layers or large layers force flush buffer
        if layer_idx % 10 == 0 || layer.add.len() + layer.mul.len() > 10000 {
            writer.flush()?;
        }

        // Write add gates
        for add_gate in &layer.add {
            // Serialize coef to appropriate string format
            let coef_str = serialize_field_element::<C>(&add_gate.coef);

            // Write add gate: Add [input_idx] [output_idx] [coef]
            writeln!(
                writer,
                "Add {} {} {}",
                add_gate.i_ids[0], add_gate.o_id, coef_str
            )?;

            items_processed += 1;
        }

        // Write mul gates
        for mul_gate in &layer.mul {
            // Serialize coef to appropriate string format
            let coef_str = serialize_field_element::<C>(&mul_gate.coef);

            // Write mul gate: Mul [input_left_idx],[input_right_idx] [output_idx] [coef]
            writeln!(
                writer,
                "Mul {},{} {} {}",
                mul_gate.i_ids[0], mul_gate.i_ids[1], mul_gate.o_id, coef_str
            )?;

            items_processed += 1;
        }

        // Flush buffer after each layer to ensure data written to disk
        writer.flush()?;

        // Calculate and display progress
        let percent = if total_items > 0 {
            (items_processed * 100) / total_items
        } else {
            100
        };
        if percent > last_percent && percent % 5 == 0 {
            println!(
                "Serialization progress: {percent}% (processed {items_processed}/{total_items} items: {total_gates} gates + {total_values} values)"
            );
            last_percent = percent;
        }
    }

    // Final flush and close file
    writer.flush()?;
    drop(writer); // Explicitly close file

    // Output first and last layer gate counts and values for verification
    if !circuit.layers.is_empty() {
        let first_layer = &circuit.layers[0];
        let last_layer = &circuit.layers[circuit.layers.len() - 1];

        println!("First layer: {} addition gates, {} multiplication gates, {} input values, {} output values", 
                 first_layer.add.len(), first_layer.mul.len(), first_layer.input_vals.len(), first_layer.output_vals.len());
        println!("Last layer: {} addition gates, {} multiplication gates, {} input values, {} output values", 
                 last_layer.add.len(), last_layer.mul.len(), last_layer.input_vals.len(), last_layer.output_vals.len());
    }

    // Verify file write success
    match std::fs::metadata(filepath) {
        Ok(metadata) => {
            // File size should be related to total items (gates + values)
            let expected_min_size = total_items * 15; // Roughly estimate each item at least 15 bytes
            if metadata.len() < expected_min_size as u64 {
                println!("Warning: file size may be insufficient, please check if fully written");
            }
            println!(
                "Successfully serialized {total_gates} gates and {total_values} values to file (total {total_items} items)"
            );
        }
        Err(e) => println!("Unable to verify file: {e}"),
    }

    Ok(())
}

/// Serialize witness as plaintext to a file
pub fn serial_circuit_witness_as_plaintext<F: FieldEngine>(
    circuit: &Circuit<F>,
    transcript: &mut impl Transcript,
    challenge: &ExpanderDualVarChallenge<F>,
) -> Result<(), SerializationError>
where
    F::CircuitField: std::fmt::Debug,
    F::SimdCircuitField: std::fmt::Debug,
{
    // Determine field type and construct filename
    let field_type = match std::mem::size_of::<F::CircuitField>() {
        32 => "bn254",
        4 => "m31ext3",
        8 => "goldilocksext2",
        _ => "unknown",
    };
    let filepath = format!("data/keccak_{field_type}.gpu.circuit");

    // Check if file already exists
    if std::path::Path::new(&filepath).exists() {
        println!("Circuit file {filepath} already exists, skipping serialization");
        return Ok(());
    }

    // Perform serialization
    println!("GPU enabled, serializing circuit to {filepath}");
    serialize_circuit_to_file(circuit, &filepath)?;
    println!("Successfully serialized circuit to {filepath}");

    // Get digest, proof bytes, and hash_start_index directly from memory using unsafe code
    let (digest_bytes, proof_bytes, hash_start_index) = unsafe {
        use gkr_hashers::SHA256hasher;
        use transcript::BytesHashTranscript;

        // Cast to BytesHashTranscript - the hasher type doesn't matter since we only access
        // digest, proof, and hash_start_index fields which have the same layout regardless of H
        let transcript_ptr = transcript as *mut _ as *mut BytesHashTranscript<SHA256hasher>;
        let bytes_transcript = &*transcript_ptr;

        (
            &bytes_transcript.digest,
            &bytes_transcript.proof.bytes,
            bytes_transcript.hash_start_index,
        )
    };

    // Check if digest has reasonable size (we expect at least 16 bytes for security)
    if digest_bytes.len() < 16 {
        panic!("Transcript digest too small: {} bytes", digest_bytes.len());
    }

    // Append to the circuit file
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&filepath)?;

    // Write transcript start marker
    writeln!(file, "=====Transcript Start=====")?;
    writeln!(file, "TranscriptDigestByte={}", digest_bytes.len())?;
    writeln!(file, "TranscriptProofByte={}", proof_bytes.len())?;
    writeln!(file, "TranscriptHashStartIndex={hash_start_index}")?;

    // Write digest bytes first
    writeln!(file, "=====Digest Bytes=====")?;
    for (i, chunk) in digest_bytes.chunks(40).enumerate() {
        let start_idx = i * 40;
        let end_idx = start_idx + chunk.len() - 1;

        // Format bytes with leading zeros and join with commas
        let formatted_bytes: Vec<String> = chunk.iter().map(|&byte| format!("{byte:03}")).collect();
        let line = formatted_bytes.join(",");

        // Write line with range annotation
        writeln!(file, "{line} //digest[{start_idx}-{end_idx}]")?;
    }

    // Write proof bytes
    writeln!(file, "=====Proof Bytes=====")?;
    for (i, chunk) in proof_bytes.chunks(40).enumerate() {
        let start_idx = i * 40;
        let end_idx = start_idx + chunk.len() - 1;

        // Format bytes with leading zeros and join with commas
        let formatted_bytes: Vec<String> = chunk.iter().map(|&byte| format!("{byte:03}")).collect();
        let line = formatted_bytes.join(",");

        // Write line with range annotation
        writeln!(file, "{line} //proof[{start_idx}-{end_idx}]")?;
    }

    // Write transcript end marker
    writeln!(file, "=====Transcript End=====")?;

    // Write the challenge
    writeln!(file, "Challenge: {challenge:?}")?;

    file.flush()?;

    println!("Transcript digest and proof bytes written to {filepath}");
    println!(
        "Total digest bytes: {}, Total proof bytes: {}, Hash start index: {}",
        digest_bytes.len(),
        proof_bytes.len(),
        hash_start_index
    );

    Ok(())
}

/// Print final claims to console (not to file)
pub fn print_final_claims<F: FieldEngine>(
    vx_claim: &F::ChallengeField,
    vy_claim: &Option<F::ChallengeField>,
) {
    println!("=====Final Claims=====");
    println!("vx_claim = {vx_claim:?}");
    if let Some(vy) = vy_claim {
        println!("vy_claim = {vy:?}");
    } else {
        println!("vy_claim = None");
    }
}
