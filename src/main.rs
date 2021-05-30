mod extract_meta;
mod edit_meta;
mod compression_stats;

/// To extract the meta data, run:
/// `cargo run --release -- extract-meta "C:\Users\Johannes\Desktop\render.exr"`
///
/// For the user, this would be `exr-tools extract-meta "C:\Users\Johannes\Desktop\render.exr"`

fn main(){
    let mut remaining_args = std::env::args().skip(1); // first is own binary path
    let tool_name = remaining_args.next().expect("expected an exr tool name"); // TODO better error handling!

    match tool_name.as_str() {
        "extract-meta" => extract_meta::main(remaining_args),
        unsupported_name => unimplemented!("exr tool with name `{}` not supported", unsupported_name)
    }
}