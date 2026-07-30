use tpack_bench::{flat_record, prepared_speedup};

fn main() {
    let (naive, prep, sref) = prepared_speedup(&flat_record(), 200_000).expect("ok");
    println!("naive FullSchema:    {naive:.1} ns/op");
    println!(
        "prepared FullSchema: {prep:.1} ns/op ({:.2}×)",
        naive / prep
    );
    println!(
        "SchemaRef warm:      {sref:.1} ns/op ({:.2}×)",
        naive / sref
    );
}
