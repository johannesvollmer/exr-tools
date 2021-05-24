
pub fn main(mut args: impl Iterator<Item=String>) {
    let path = args.next().expect("no path specified"); // TODO better error handling!
    println!("loading image from path {}", path);

    let meta = exr::meta::MetaData::read_from_file(path, false).unwrap();

    // print the stuff
    // TODO make it nice (I suspect Narann already crafted some code for this? haha)
    println!("{:#?}", meta);
}