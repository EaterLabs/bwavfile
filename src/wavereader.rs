
use std::fs::File;

use super::parser::Parser;
use super::fourcc::{FourCC, FMT__SIG,DATA_SIG, BEXT_SIG, JUNK_SIG, FLLR_SIG};
use super::errors::Error as ParserError;
use super::raw_chunk_reader::RawChunkReader;
use super::fmt::WaveFmt;
use super::bext::Bext;
use super::audio_frame_reader::AudioFrameReader;
use super::chunks::ReadBWaveChunks;


use std::io::{Read, Seek};


/**
 * Wave, Broadcast-WAV and RF64/BW64 parser/reader.
 * 
 * ```
 * use bwavfile::WaveReader;
 * let mut r = WaveReader::open("tests/media/ff_silence.wav").unwrap();
 * 
 * let format = r.format().unwrap();
 * assert_eq!(format.sample_rate, 44100);
 * assert_eq!(format.channel_count, 1);
 * 
 * let mut frame_reader = r.audio_frame_reader().unwrap();
 * let mut buffer = frame_reader.create_frame_buffer();
 * 
 * let read = frame_reader.read_integer_frame(&mut buffer).unwrap();
 * 
 * assert_eq!(buffer, [0i32]);
 * assert_eq!(read, 1);
 * 
 * ```
*/
#[derive(Debug)]
pub struct WaveReader<R: Read + Seek> {
    pub inner: R,
}

impl WaveReader<File> {
    /**
     * Open a file for reading.
     * 
     * A convenience that opens `path` and calls `Self::new()`
     *   
     */
    pub fn open(path: &str) -> Result<Self, ParserError> {
        let inner = File::open(path)?;
        return Ok( Self::new(inner)? )
    }
}

impl<R: Read + Seek> WaveReader<R> {
    /**
     * Wrap a `Read` struct in a new `WaveReader`.
     * 
     * This is the primary entry point into the `WaveReader` interface. The
     * stream passed as `inner` must be at the beginning of the header of the
     * WAVE data. For a .wav file, this means it must be at the start of the 
     * file.
     * 
     * This function does a minimal validation on the provided stream and
     * will return an `Err(errors::Error)` immediately if there is a structural 
     * inconsistency that makes the stream unreadable or if it's missing 
     * essential components that make interpreting the audio data impoossible.
     * 
     * ```rust
     * use std::fs::File;
     * use std::io::{Error,ErrorKind};
     * use bwavfile::{WaveReader, Error as WavError};
     * 
     * let f = File::open("tests/media/error.wav").unwrap();
     * 
     * let reader = WaveReader::new(f);
     * 
     * match reader {
     *      Ok(_) => panic!("error.wav should not be openable"),
     *      Err( WavError::IOError( e ) ) => {
     *          assert_eq!(e.kind(), ErrorKind::UnexpectedEof)
     *      }
     *      Err(e) => panic!("Unexpected error was returned {:?}", e)
     * }
     * 
     * ```
     * 
    */
    pub fn new(inner: R) -> Result<Self,ParserError> {
        let mut retval = Self { inner };
        retval.validate_readable()?;
        Ok(retval)
    }

    /**
     * Unwrap the inner reader.
     */
    pub fn into_inner(self) -> R {
        return self.inner;
    }

    /**
     * Create an `AudioFrameReader` for reading each audio frame.
    */
    pub fn audio_frame_reader(&mut self) -> Result<AudioFrameReader<RawChunkReader<R>>, ParserError> {
        let format = self.format()?;
        let audio_chunk_reader = self.chunk_reader(DATA_SIG, 0)?;
        Ok(AudioFrameReader::new(audio_chunk_reader, format))
    }

    /**
     * The count of audio frames in the file.
     */
    pub fn frame_length(&mut self) -> Result<u64, ParserError> {
        let (_, data_length ) = self.get_chunk_extent_at_index(DATA_SIG, 0)?;
        let format = self.format()?;
        Ok( data_length / (format.block_alignment as u64) )
    } 

    /**
     * Sample and frame format of this wave file.
     */
    pub fn format(&mut self) -> Result<WaveFmt, ParserError> {
        self.chunk_reader(FMT__SIG, 0)?.read_wave_fmt()
    }

    /**
     * The Broadcast-WAV metadata record for this file.
     */
    pub fn broadcast_extension(&mut self) -> Result<Bext, ParserError> {
        self.chunk_reader(BEXT_SIG, 0)?.read_bext()
    }

    /**
    * Validate file is readable.
    * 
    *  `Ok(())` if the source meets the minimum standard of 
    *  readability by a permissive client:
    *  - `fmt` chunk and `data` chunk are present
    *  - `fmt` chunk appears before `data` chunk
    */
    pub fn validate_readable(&mut self) -> Result<(), ParserError> {
        let (fmt_pos, _)  = self.get_chunk_extent_at_index(FMT__SIG, 0)?;
        let (data_pos, _) = self.get_chunk_extent_at_index(DATA_SIG, 0)?;

        if fmt_pos < data_pos {
            Ok(())
        } else {
            Err( ParserError::FmtChunkAfterData)
        }
    }

    /** 
     * Validate minimal WAVE file.
     * 
     * `Ok(())` if the source is `validate_readable()` AND
     * 
     *   - Contains _only_ a `fmt` chunk and `data` chunk, with no other chunks present
     *   - is not an RF64/BW64
     * 
     * Some clients require a WAVE file to only contain format and data without any other
     * metadata and this function is provided to validate this condition.
     * 
     * ### Examples
     * 
     * ```
     * # use bwavfile::WaveReader;
     * 
     * let mut w = WaveReader::open("tests/media/ff_minimal.wav").unwrap();
     * w.validate_minimal().expect("Minimal wav did not validate not minimal!");
     * ```
     * 
     * ```
     * # use bwavfile::WaveReader;
     * 
     * let mut x = WaveReader::open("tests/media/pt_24bit_51.wav").unwrap();
     * x.validate_minimal().expect_err("Complex WAV validated minimal!");
     * ```
    */
    pub fn validate_minimal(&mut self) -> Result<(), ParserError>  {
        self.validate_readable()?;

        let chunk_fourccs : Vec<FourCC> = Parser::make(&mut self.inner)?
            .into_chunk_list()?.iter().map(|c| c.signature ).collect();

        if chunk_fourccs == vec![FMT__SIG, DATA_SIG] {
            Ok(())
        } else {
            Err( ParserError::NotMinimalWaveFile )
        }
    }

    /**
     * Validate Broadcast-WAVE file format
     * 
     * Returns `Ok(())` if `validate_readable()` and file contains a 
     * Broadcast-WAV metadata record (a `bext` chunk).
     * 
     * ### Examples
     * 
     * ```
     * # use bwavfile::WaveReader;
     * 
     * let mut w = WaveReader::open("tests/media/ff_bwav_stereo.wav").unwrap();
     * w.validate_broadcast_wave().expect("BWAVE file did not validate BWAVE");
     * 
     * let mut x = WaveReader::open("tests/media/pt_24bit.wav").unwrap();
     * x.validate_broadcast_wave().expect("BWAVE file did not validate BWAVE");
     * 
     * let mut y = WaveReader::open("tests/media/audacity_16bit.wav").unwrap();
     * y.validate_broadcast_wave().expect_err("Plain WAV file DID validate BWAVE");
     * ```
    */
    pub fn validate_broadcast_wave(&mut self) -> Result<(), ParserError> {
        self.validate_readable()?;
        let (_, _) = self.get_chunk_extent_at_index(BEXT_SIG, 0)?;
        Ok(())
    } 

    /**
     * Verify data is aligned to a block boundary.
     * 
     * Returns `Ok(())` if `validate_readable()` and the start of the 
     * `data` chunk's content begins at 0x4000.
    */
    pub fn validate_data_chunk_alignment(&mut self) -> Result<() , ParserError> {
        self.validate_readable()?;
        let (start, _) = self.get_chunk_extent_at_index(DATA_SIG, 0)?;
        if start == 0x4000 {
            Ok(())
        } else {
            Err(ParserError::DataChunkNotAligned)
        }
    }

    /**
     * Verify audio data can be appended immediately to this file.
     * 
     * Returns `Ok(())` if:
     *  - `validate_readable()`
     *  - there is a `JUNK` or `FLLR` immediately at the beginning of the chunk 
     *    list adequately large enough to be overwritten by a `ds64` (92 bytes)
     *  - `data` is the final chunk
    */
    pub fn validate_prepared_for_append(&mut self) -> Result<(), ParserError> {
        self.validate_readable()?;

        let chunks = Parser::make(&mut self.inner)?.into_chunk_list()?;
        let ds64_space_required = 92;

        let eligible_filler_chunks = chunks.iter()
            .take_while(|c| c.signature == JUNK_SIG || c.signature == FLLR_SIG);

        let filler = eligible_filler_chunks
            .enumerate()
            .fold(0, |accum, (n, item)| if n == 0 { accum + item.length } else {accum + item.length + 8});

        if filler < ds64_space_required {
            Err(ParserError::InsufficientDS64Reservation {expected: ds64_space_required, actual: filler})
        } else {
            let data_pos = chunks.iter().position(|c| c.signature == DATA_SIG);
        
            match data_pos {
                Some(p) if p == chunks.len() - 1 => Ok(()),
                _ => Err(ParserError::DataChunkNotPreparedForAppend)
            }
        }
    }
}

impl<R:Read+Seek> WaveReader<R> { /* Private Implementation */

    fn chunk_reader(&mut self, signature: FourCC, at_index: u32) -> Result<RawChunkReader<R>, ParserError> {
        let (start, length) = self.get_chunk_extent_at_index(signature, at_index)?;
        Ok( RawChunkReader::new(&mut self.inner, start, length) )
    } 

    fn get_chunk_extent_at_index(&mut self, fourcc: FourCC, index: u32) -> Result<(u64,u64), ParserError> {
        let p = Parser::make(&mut self.inner)?.into_chunk_list()?;

        if let Some(chunk) = p.iter().filter(|item| item.signature == fourcc).nth(index as usize) {
            Ok ((chunk.start, chunk.length))
        } else {
            Err( ParserError::ChunkMissing { signature : fourcc })
        }
    }
}
