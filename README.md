[![Crates.io](https://img.shields.io/crates/l/bwavfile)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/bwavfile)](https://crates.io/crates/bwavfile/)
![GitHub last commit](https://img.shields.io/github/last-commit/iluvcapra/bwavfile)
[![GitHub Workflow Status](https://img.shields.io/github/workflow/status/iluvcapra/bwavfile/Rust)](https://github.com/iluvcapra/bwavfile/actions?query=workflow%3ARust)

# bwavfile
Rust Wave File Reader/Writer with Broadcast-WAV, MBWF and RF64 Support

This is currently a work-in-progress!

## Use

```rust

 use bwavfile::WaveReader;
 let mut r = WaveReader::open("tests/media/ff_silence.wav").unwrap();
 
 let format = r.format().unwrap();
 assert_eq!(format.sample_rate, 44100);
 assert_eq!(format.channel_count, 1);
 
 let mut frame_reader = r.audio_frame_reader().unwrap();
 let mut buffer = frame_reader.create_frame_buffer();
 
 let read = frame_reader.read_integer_frame(&mut buffer).unwrap();
 
 assert_eq!(buffer, [0i32]);
 assert_eq!(read, 1);
```

## Note on Testing

All of the media for the integration tests is committed to the respository
in zipped form. Before you can run tests, you need to `cd` into the `tests` 
directory and run the `create_test_media.sh` script. Note that one of the 
test files (the RF64 test case) is over four gigs in size.

## Resources

 
### Implementation of Broadcast Wave Files
  - [EBU Tech 3285][ebu3285] (May 2011), "Specification of the Broadcast Wave Format (BWF)"


### Implementation of 64-bit Wave Files
  - [ITU-R 2088][itu2088] (October 2019), "Long-form file format for the international exchange of audio programme materials with metadata"
    - Presently in force, adopted by the EBU in [EBU Tech 3306v2][ebu3306v2] (June 2018).
  - [EBU Tech 3306v1][ebu3306v1] (July 2009), "MBWF / RF64: An extended File Format for Audio"
    - No longer in force, however long-established. 


### Implementation of Wave format `fmt` chunk
  - [MSDN WAVEFORMATEX](https://docs.microsoft.com/en-us/windows/win32/api/mmeapi/ns-mmeapi-waveformatex)
  - [MSDN WAVEFORMATEXTENSIBLE](https://docs.microsoft.com/en-us/windows/win32/api/mmreg/ns-mmreg-waveformatextensible)


### Other resources
- [RFC 3261][rfc3261] (June 1998) "WAVE and AVI Codec Registries" 
- [Peter Kabal, McGill University](http://www-mmsp.ece.mcgill.ca/Documents/AudioFormats/WAVE/WAVE.html)
  - [Multimedia Programming Interface and Data Specifications 1.0](http://www-mmsp.ece.mcgill.ca/Documents/AudioFormats/WAVE/Docs/riffmci.pdf) 
    IBM Corporation and Microsoft Corporation, (August 1991)

[ebu3285]:  https://tech.ebu.ch/docs/tech/tech3285.pdf
[ebu3306v1]: https://tech.ebu.ch/docs/tech/tech3306v1_1.pdf
[ebu3306v2]:  https://tech.ebu.ch/docs/tech/tech3306.pdf
[itu2088]:  https://www.itu.int/dms_pubrec/itu-r/rec/bs/R-REC-BS.2088-1-201910-I!!PDF-E.pdf
[rfc3261]:  https://tools.ietf.org/html/rfc2361
