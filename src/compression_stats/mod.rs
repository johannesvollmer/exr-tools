use exr::image::Image;
use exr::compression::Compression;
use std::time::Duration;
use std::io::{ErrorKind, SeekFrom};
use std::io::Write;
use std::borrow::BorrowMut;

pub fn main(mut args: impl Iterator<Item=String>){
    let path = args.next().expect("first arg must be image file path");

    println!("warning: measured timing only applies to the Rust implementation, \
            not to the (likely faster) reference implementation.");

    println!("analyzing exr file `{}`...", path);

    use exr::prelude::*;
    let mut image = read_all_data_from_file(path).expect("image cannot be opened");



    struct NullWriter {
        pos: u64,
        current_byte_size: u64,
    }

    impl Write for NullWriter {
        fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            self.pos = self.pos.checked_add(buffer.len() as u64)
                .ok_or_else(|| std::io::Error::new(ErrorKind::OutOfMemory, "file too large"))?;

            self.current_byte_size = self.current_byte_size.max(self.pos);
            Ok(buffer.len())
        }
    }

    impl std::io::Seek for NullWriter {
        fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
            match pos {
                SeekFrom::Start(pos) => self.pos = pos,
                SeekFrom::Current(advance) => self.pos = (self.pos as i64 + advance) as u64, // TODO errors
                SeekFrom::End(back) => self.pos = (self.current_byte_size as i64 + back) as u64,
            };

            Ok(self.pos)
        }
    }

    #[derive(Clone, Copy, PartialEq)]
    struct Stats {
        compression: Compression,
        duration: Duration,
        byte_size: u64,
    }

    let all_compressions = vec![
        Compression::Uncompressed,
        Compression::RLE,
        Compression::PIZ,
        Compression::ZIP1,
        Compression::ZIP16,
        Compression::PXR24,
        Compression::B44,
        Compression::B44A,
    ];

    let lossless_compressions = all_compressions.iter().cloned()
        .filter(|compr| !compr.may_loose_data())
        .collect::<Vec<_>>();

    let stats = image.layer_data.iter_mut().map(|layer|{
        let mut compressions = lossless_compressions.clone();
        if !compressions.contains(&layer.encoding.compression) { compressions.push(layer.encoding.compression); }

        let stats = compressions.iter().map(|&compression| {
            let start_time = std::time::Instant::now();
            let mut writer = NullWriter { current_byte_size: 0, pos: 0, };

            let mut new_image = Image::from_layer(layer.clone());
            new_image.layer_data.encoding.compression = compression;
            new_image.write().to_buffered(writer.borrow_mut())?;

            let end_time = std::time::Instant::now();
            let duration = end_time - start_time;

            Ok(Stats {
                compression, duration,
                byte_size: writer.current_byte_size,
            })

        }).collect::<Vec<Result<Stats>>>();

        (layer, stats)
    });

    for (index, (layer, stats)) in stats.enumerate() {
        println!(
            "\nLayer #{}{}:",
             index,

            layer.attributes.layer_name.as_ref()
                .map(|name| format!(" (`{}`)", name))
                .unwrap_or_default(),
        );

        let smallest = stats.iter().flatten().min_by_key(|stat| stat.byte_size)
            .expect("need more than 1 compression method to find smallest");

        let fastest = stats.iter().flatten().min_by_key(|stat| stat.duration)
            .expect("need more than 1 compression method to fastest");

        let current = stats.iter().flatten()
            .find(|stat| stat.compression == layer.encoding.compression)
            .expect("need more than 1 compression method");

        let best = {
            let mut remaining = stats.iter().flatten().cloned().collect::<Vec<_>>();

            loop {
                if remaining.len() <= 1 { break; }
                let largest = *remaining.iter()
                    .max_by_key(|stat| stat.byte_size).unwrap();

                remaining.retain(|item| *item != largest);

                if remaining.len() <= 1 { break; }
                let slowest = *remaining.iter()
                    .max_by_key(|stat| stat.duration).unwrap();

                remaining.retain(|item| *item != slowest);
            }

            remaining.into_iter().next()
                .expect("at least one compression method required")
        };


        println!(
            "current: {}, {}b, {}s",
            current.compression,
            current.byte_size,
            current.duration.as_secs_f32(),
        );

        println!(
            "probably best: {}, saving {:.1}% memory and {:.1}% time",
            best.compression,
            (1.0 - best.byte_size as f32 / current.byte_size as f32) * 100.0,
            (1.0 - best.duration.as_secs_f32() / current.duration.as_secs_f32()) * 100.0
        );

        println!(
            "smallest: {}, {}b, saving {:.1}% memory",
            smallest.compression, smallest.byte_size,
            (1.0 - smallest.byte_size as f32 / current.byte_size as f32) * 100.0
        );

        println!(
            "fastest: {}, {}s, saving {:.1}% compression time",
            fastest.compression, fastest.duration.as_secs_f32(),
            (1.0 - fastest.duration.as_secs_f32() / current.duration.as_secs_f32()) * 100.0
        );

        println!("full stats:");
        for stat in stats {
            match stat {
                Ok(stat) => println!("\t{}: \t\t\t{}b, \t\t\t{}s", stat.compression, stat.byte_size, stat.duration.as_secs_f32()),
                Err(error) => println!("\t{}", error)
            }
        }

        let apply_best = true;
        if apply_best { layer.encoding.compression = best.compression; }
    }

}