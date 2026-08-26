use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=support-matrix.csv");
    let csv = fs::read_to_string("support-matrix.csv").expect("read support-matrix.csv");
    let mut lines = csv.lines();
    let header = lines.next().expect("support matrix header");
    assert_eq!(
        header,
        "version,snapshot,version_date,phase,variant,namespace,schema_root,fixture,typed_module"
    );

    let mut generated = String::from("pub static SUPPORT_MATRIX: &[SupportEntry] = &[\n");
    for (index, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split(',').collect();
        assert_eq!(fields.len(), 9, "support matrix row {}", index + 2);
        let version = match fields[0] {
            "3.1" => "GaebVersion::V3_1",
            "3.2" => "GaebVersion::V3_2",
            value => panic!("unsupported version {value:?} in support matrix"),
        };
        let phase = match (fields[3], fields[4]) {
            ("81", "") => "ExchangePhase::X81",
            ("83", "") => "ExchangePhase::X83",
            ("83", "z") => "ExchangePhase::X83Z",
            ("84", "") => "ExchangePhase::X84",
            ("84", "z") => "ExchangePhase::X84Z",
            ("86", "") => "ExchangePhase::X86",
            ("86", "ze") => "ExchangePhase::X86ZE",
            ("86", "zr") => "ExchangePhase::X86ZR",
            value => panic!("unsupported phase/variant {value:?} in support matrix"),
        };
        let variant = if fields[4].is_empty() {
            "None".to_owned()
        } else {
            format!("Some({:?})", fields[4])
        };
        let typed_module = if fields[8].is_empty() {
            "None".to_owned()
        } else {
            format!("Some({:?})", fields[8])
        };
        generated.push_str(&format!(
            "SupportEntry {{ version: {version}, snapshot: {:?}, version_date: {:?}, phase: {phase}, variant: {variant}, namespace: {:?}, schema_root: {:?}, fixture: {:?}, typed_module: {typed_module} }},\n",
            fields[1], fields[2], fields[5], fields[6], fields[7]
        ));
    }
    generated.push_str("];\n");

    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("support_matrix.rs");
    fs::write(output, generated).expect("write generated support matrix");
}
