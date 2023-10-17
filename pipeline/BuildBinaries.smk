

rule build_variable_binary:
    threads: 1
    resources:
             cargo=1
    input:
         cargo="../column_generation/src/bin/solution_approach_variable.rs"
    output:
          OUTPUT_PREFIX + "/binaries/solution_approach_variable"
    shell:
         "RUSTFLAGS='-C link-arg=-s -C target-cpu=native' cargo +nightly build -Z unstable-options -j 1 --profile=cluster --features='snakemake' --manifest-path ../column_generation/Cargo.toml --no-default-features --bin solution_approach_variable --target-dir {OUTPUT_PREFIX}/target/ --out-dir $(dirname {output})"

rule build_simulation_binary:
    threads: 1
    resources:
             cargo=1
    input:
         cargo="../column_generation/src/bin/simulation_feasible.rs"
    output:
          OUTPUT_PREFIX + "/binaries/simulation_feasible"
    shell:
         "RUSTFLAGS='-C link-arg=-s -C target-cpu=native' cargo +nightly build -Z unstable-options -j 1 --profile=cluster --features='snakemake' --manifest-path ../column_generation/Cargo.toml --no-default-features --bin simulation_feasible --target-dir {OUTPUT_PREFIX}/target/ --out-dir $(dirname {output})"



rule build_check_feasibility_binary:
    threads: 1
    resources:
             cargo=1
    input:
        cargo="../column_generation/src/bin/check_feasibility.rs"
    output:
        OUTPUT_PREFIX + "/binaries/check_feasibility"
    shell:
        "RUSTFLAGS='-C link-arg=-s -C target-cpu=native' cargo +nightly build -Z unstable-options -j 1 --profile=cluster --features='snakemake' --manifest-path ../column_generation/Cargo.toml --no-default-features --bin check_feasibility --target-dir {OUTPUT_PREFIX}/target/ --out-dir $(dirname {output})"

rule build_check_cross_feasibility_binary:
    threads: 1
    resources:
             cargo=1
    input:
        cargo="../column_generation/src/bin/check_cross_feasibility.rs"
    output:
        OUTPUT_PREFIX + "/binaries/check_cross_feasibility"
    shell:
        "RUSTFLAGS='-C link-arg=-s -C target-cpu=native' cargo +nightly build -Z unstable-options -j 1 --profile=cluster --features='snakemake' --manifest-path ../column_generation/Cargo.toml --no-default-features --bin check_cross_feasibility --target-dir {OUTPUT_PREFIX}/target/ --out-dir $(dirname {output})"

rule build_remove_infeasible_binary:
    threads: 1
    resources:
             cargo=1
    input:
        cargo="../column_generation/src/bin/remove_infeasible.rs"
    output:
        OUTPUT_PREFIX + "/binaries/remove_infeasible"
    shell:
        "RUSTFLAGS='-C link-arg=-s -C target-cpu=native' cargo +nightly build -Z unstable-options -j 1 --profile=cluster --features='snakemake' --manifest-path ../column_generation/Cargo.toml --no-default-features --bin remove_infeasible --target-dir {OUTPUT_PREFIX}/target/ --out-dir $(dirname {output})"
