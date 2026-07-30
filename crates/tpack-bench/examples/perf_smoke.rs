//! Tiny prepared-vs-naive check (not the full matrix).
use tpack_bench::{flat_record, prepared_speedup};

fn main() {
    let w = flat_record();
    let s = prepared_speedup(&w, 200_000).expect("speedup");
    println!("naive FullSchema:     {:.1} ns/op", s.naive_ns);
    println!(
        "prepared FullSchema:  {:.1} ns/op ({:.2}×)",
        s.prepared_ns,
        s.naive_ns / s.prepared_ns
    );
    println!(
        "SchemaRef warm:       {:.1} ns/op ({:.2}×)",
        s.schemaref_ns,
        s.naive_ns / s.schemaref_ns
    );
}
