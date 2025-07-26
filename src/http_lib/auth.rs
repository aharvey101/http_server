// Simple base64 decoder for authentication (simplified implementation)
pub fn base64_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = Vec::new();
    let input = input.trim();
    
    if input.len() % 4 != 0 {
        return Err("Invalid base64 length");
    }
    
    for chunk in input.as_bytes().chunks(4) {
        let mut values = [0u8; 4];
        
        for (i, &byte) in chunk.iter().enumerate() {
            if byte == b'=' {
                values[i] = 0;
            } else if let Some(pos) = CHARS.iter().position(|&c| c == byte) {
                values[i] = pos as u8;
            } else {
                return Err("Invalid base64 character");
            }
        }
        
        let combined = ((values[0] as u32) << 18) | 
                      ((values[1] as u32) << 12) | 
                      ((values[2] as u32) << 6) | 
                      (values[3] as u32);
        
        result.push((combined >> 16) as u8);
        if chunk[2] != b'=' {
            result.push((combined >> 8) as u8);
        }
        if chunk[3] != b'=' {
            result.push(combined as u8);
        }
    }
    
    Ok(result)
}
