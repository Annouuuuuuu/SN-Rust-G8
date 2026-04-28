use crate::errors::{FileSentinelError, Result};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use log::{info, debug};

pub struct CompressionManager {
    level: u32,
    min_size: u64,
    temp_dir: PathBuf,
}

impl CompressionManager {
    pub fn new(temp_dir: PathBuf, level: u32, min_size: u64) -> Result<Self> {
        fs::create_dir_all(&temp_dir)?;
        
        let level = level.min(9).max(1);
        
        Ok(Self {
            level,
            min_size,
            temp_dir,
        })
    }

    /// Compresse un fichier
    pub fn compress_file<P: AsRef<Path>>(&self, source: P) -> Result<PathBuf> {
        let source = source.as_ref();
        
        // Vérifier si la compression est nécessaire
        let metadata = fs::metadata(source)?;
        if metadata.len() < self.min_size {
            debug!(
                "File too small for compression: {} ({} bytes)",
                source.display(),
                metadata.len()
            );
            return Ok(source.to_path_buf());
        }

        let compressed_filename = format!(
            "{}_compressed.gz",
            source.file_name().unwrap().to_string_lossy()
        );
        let compressed_path = self.temp_dir.join(compressed_filename);

        // Lire le fichier source
        let source_content = fs::read(source).map_err(|e| {
            FileSentinelError::Compression(format!(
                "Cannot read file {}: {}",
                source.display(),
                e
            ))
        })?;

        // Compresser
        let mut encoder = GzEncoder::new(
            fs::File::create(&compressed_path)?,
            Compression::new(self.level),
        );

        encoder.write_all(&source_content)?;
        encoder.finish()?;

        let compressed_size = fs::metadata(&compressed_path)?.len();
        let ratio = compressed_size as f64 / metadata.len() as f64 * 100.0;

        debug!(
            "Compressed: {} -> {} ({:.1}% of original)",
            source.display(),
            compressed_path.display(),
            ratio
        );

        Ok(compressed_path)
    }

    /// Décompresse un fichier
    pub fn decompress_file<P: AsRef<Path>>(&self, compressed: P, output: P) -> Result<()> {
        let compressed = compressed.as_ref();
        let output = output.as_ref();

        // Lire le fichier compressé
        let compressed_content = fs::read(compressed)?;
        let mut decoder = GzDecoder::new(&compressed_content[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        // Créer le répertoire parent si nécessaire
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }

        // Écrire le fichier décompressé
        fs::write(output, decompressed)?;

        debug!(
            "Decompressed: {} -> {}",
            compressed.display(),
            output.display()
        );

        Ok(())
    }

    /// Nettoie les fichiers temporaires
    pub fn cleanup(&self) -> Result<()> {
        if self.temp_dir.exists() {
            for entry in fs::read_dir(&self.temp_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    fs::remove_file(path)?;
                }
            }
        }
        Ok(())
    }

    /// Calcule le ratio de compression
    pub fn compression_ratio<P: AsRef<Path>>(&self, original: P, compressed: P) -> Result<f64> {
        let original_size = fs::metadata(original)?.len() as f64;
        let compressed_size = fs::metadata(compressed)?.len() as f64;

        if original_size > 0.0 {
            Ok(compressed_size / original_size * 100.0)
        } else {
            Ok(100.0)
        }
    }
}