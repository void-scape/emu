use std::process::Command;

fn main() {
    let _ = std::fs::create_dir("./emu/build");

    for file in std::fs::read_dir("./emu").unwrap() {
        let entry = file.unwrap();
        if let Some(ext) = entry.path().extension() {
            if ext.to_str().unwrap() == "cpp" {
                let fname = entry.path();
                let fname = fname.file_stem().unwrap();

                let output = Command::new("riscv64-elf-gcc")
                    .arg("-S")
                    .arg("-nostdlib")
                    .arg("-fno-exceptions")
                    .arg("-fno-rtti")
                    .arg("-ffreestanding")
                    .arg("-march=rv64g")
                    .arg("-Wall")
                    .arg("-O3")
                    .arg("-o")
                    .arg(format!("./emu/build/{}.s", fname.to_str().unwrap()))
                    .arg(entry.path())
                    .output()
                    .unwrap();

                if !output.stdout.is_empty() {
                    println!("compiler: {}", String::from_utf8_lossy(&output.stdout));
                }
                if !output.stderr.is_empty() {
                    println!("compiler: {}", String::from_utf8_lossy(&output.stderr));
                }
            }
        }
    }

    let sfiles = {
        let mut sfiles = Vec::new();
        for file in std::fs::read_dir("./emu/build").unwrap() {
            let entry = file.unwrap();
            if let Some(ext) = entry.path().extension() {
                if ext.to_str().unwrap() == "s" {
                    sfiles.push(entry.path());
                }
            }
        }
        sfiles
    };

    let mut output = Command::new("riscv64-elf-gcc");
    output
        .arg("-nostdlib")
        .arg("-emain")
        .arg("-ffreestanding")
        .arg("-Wl,-Tmain_linker.ld")
        .arg("-march=rv64g")
        .arg("-o")
        .arg("./emu/build/emu.o");
    for arg in sfiles.iter() {
        output.arg(arg);
    }
    let output = output.output().unwrap();
    if !output.stdout.is_empty() {
        println!("assembler: {}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        println!("assembler: {}", String::from_utf8_lossy(&output.stderr));
    }

    let output = Command::new("riscv64-elf-objcopy")
        .arg("-O")
        .arg("binary")
        .arg("./emu/build/emu.o")
        .arg("./emu/build/emu")
        //.arg("./emu/build/pgrm")
        .output()
        .unwrap();
    if !output.stdout.is_empty() {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        println!("{}", String::from_utf8_lossy(&output.stderr));
    }

    let objdump_output = Command::new("riscv64-elf-objdump")
        .arg("-d")
        .arg("./emu/build/emu.o")
        .output()
        .unwrap();

    println!(
        "\nDisassembly:\n{}\n",
        String::from_utf8_lossy(&objdump_output.stdout)
    );

    let raw = std::fs::read("./emu/build/emu").unwrap();
    emu::emulator::run_emulator(&raw);

    std::fs::remove_dir_all("./emu/build").unwrap();
}
