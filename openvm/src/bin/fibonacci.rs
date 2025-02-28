// ANCHOR: dependencies
use std::{fs, sync::Arc};
use std::time::{Duration, Instant};

use eyre::Result;
use openvm::platform::memory::MEM_SIZE;
use openvm_build::GuestOptions;
use openvm_sdk::{
    config::{AppConfig, SdkVmConfig},
    prover::AppProver,
    Sdk, StdIn,
};
use openvm_stark_sdk::config::FriParameters;
use openvm_transpiler::elf::Elf;
use serde::{Deserialize, Serialize};
use utils::{benchmark, size};

#[derive(Serialize, Deserialize)]
pub struct SomeStruct {
    pub a: u64,
    pub b: u64,
}
// ANCHOR_END: dependencies

type BenchResult = (Duration, usize, usize);

#[allow(dead_code, unused_variables)]
fn read_elf() -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: read_elf
    // 2b. Load the ELF from a file
    let elf_bytes = fs::read("your_path_to_elf").unwrap();
    let elf = Elf::decode(&elf_bytes, MEM_SIZE as u32).unwrap();
    // ANCHOR_END: read_elf
    Ok(())
}

#[allow(unused_variables, unused_doc_comments)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
  let ns = [10, 50, 90];
  benchmark(
    benchmark_fib,
    &ns,
    "../benchmark_outputs/fib_openvm.csv",
    "n",
  );

    Ok(())
}

fn benchmark_fib(n: u32) -> BenchResult {
      // ANCHOR: vm_config
      let vm_config = SdkVmConfig::builder()
      .system(Default::default())
      .rv32i(Default::default())
      .rv32m(Default::default())
      .io(Default::default())
      .build();
  // ANCHOR_END: vm_config

  /// to import example guest code in crate replace `target_path` for:
  /// ```
  /// use std::path::PathBuf;
  ///
  /// let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).to_path_buf();
  /// path.push("guest");
  /// let target_path = path.to_str().unwrap();
  /// ```
  // ANCHOR: build
  // 1. Build the VmConfig with the extensions needed.
  let sdk = Sdk;

  // 2a. Build the ELF with guest options and a target filter.
  let guest_opts = GuestOptions::default();
  let target_path = "fibonacci-guest";
  let elf = sdk.build(guest_opts, target_path, &Default::default()).unwrap();
  // ANCHOR_END: build

  // ANCHOR: transpilation
  // 3. Transpile the ELF into a VmExe
  let exe = sdk.transpile(elf, vm_config.transpiler()).unwrap();
  // ANCHOR_END: transpilation

  // ANCHOR: execution
  // 4. Format your input into StdIn
  let my_input = SomeStruct { a: 1, b: 2 }; // anything that can be serialized
  let mut stdin = StdIn::default();
  stdin.write(&n);

  // 5. Run the program
  let output = sdk.execute(exe.clone(), vm_config.clone(), stdin.clone()).unwrap();
  println!("public values output: {:?}", output);
  // ANCHOR_END: execution

  // ANCHOR: proof_generation
  // 6. Set app configuration
  let app_log_blowup = 2;
  let app_fri_params = FriParameters::standard_with_100_bits_conjectured_security(app_log_blowup);
  let app_config = AppConfig::new(app_fri_params, vm_config);

  // 7. Commit the exe
  let app_committed_exe = sdk.commit_app_exe(app_fri_params, exe).unwrap();

  // 8. Generate an AppProvingKey
  let app_pk = Arc::new(sdk.app_keygen(app_config).unwrap());

  // 9a. Generate a proof
  let proof = sdk.generate_app_proof(app_pk.clone(), app_committed_exe.clone(), stdin.clone()).unwrap();
  // 9b. Generate a proof with an AppProver with custom fields
  let start = Instant::now();
  let app_prover = AppProver::new(app_pk.app_vm_pk.clone(), app_committed_exe.clone())
      .with_program_name("test_program");
  let proof = app_prover.generate_app_proof(stdin.clone());
  // ANCHOR_END: proof_generation
  let end = Instant::now();

  // ANCHOR: verification
  // 10. Verify your program
  let app_vk = app_pk.get_app_vk();
  sdk.verify_app_proof(&app_vk, &proof).unwrap();
  // ANCHOR_END: verification

  (
    end.duration_since(start),
    size(&proof),
    0x0,
  )
}
