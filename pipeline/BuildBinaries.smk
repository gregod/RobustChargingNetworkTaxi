

rule build_benders_binary:
    threads: 1
    resources:
             cargo=1
    input:
         cargo="/home/gregor/Code/et/column_generation/src/bin/benders.rs"
    output:
          OUTPUT_PREFIX + "/binaries/benders"
    shell:
         "RUSTFLAGS='-C link-arg=-s -C target-cpu=native' cargo +nightly build -Z unstable-options -j 1 --profile=cluster --features='snakemake' --manifest-path /home/gregor/Code/et/column_generation/Cargo.toml --no-default-features --bin benders --target-dir {OUTPUT_PREFIX}/target/ --out-dir $(dirname {output})"


rule build_robust2_binary:
    threads: 1
    resources:
             cargo=1
    input:
         cargo="/home/gregor/Code/et/column_generation/src/bin/robust2.rs"
    output:
          OUTPUT_PREFIX + "/binaries/robust2"
    shell:
         "RUSTFLAGS='-C link-arg=-s -C target-cpu=native' cargo +nightly build -Z unstable-options -j 1 --profile=cluster --features='snakemake' --manifest-path /home/gregor/Code/et/column_generation/Cargo.toml --no-default-features --bin robust2 --target-dir {OUTPUT_PREFIX}/target/ --out-dir $(dirname {output})"


rule build_robust_iis_binary:
    threads: 1
    resources:
             cargo=1
    input:
         cargo="/home/gregor/Code/et/column_generation/src/bin/robust_iis.rs"
    output:
          OUTPUT_PREFIX + "/binaries/robust_iis"
    shell:
         "RUSTFLAGS='-C link-arg=-s -C target-cpu=native' cargo +nightly build -Z unstable-options -j 1 --profile=cluster --features='snakemake' --manifest-path /home/gregor/Code/et/column_generation/Cargo.toml --no-default-features --bin robust_iis --target-dir {OUTPUT_PREFIX}/target/ --out-dir $(dirname {output})"


rule build_check_feasibility_binary:
    threads: 1
    resources:
             cargo=1
    input:
        cargo="/home/gregor/Code/et/column_generation/src/bin/check_feasibility.rs"
    output:
        OUTPUT_PREFIX + "/binaries/check_feasibility"
    shell:
        "RUSTFLAGS='-C link-arg=-s -C target-cpu=native' cargo +nightly build -Z unstable-options -j 1 --profile=cluster --features='snakemake' --manifest-path /home/gregor/Code/et/column_generation/Cargo.toml --no-default-features --bin check_feasibility --target-dir {OUTPUT_PREFIX}/target/ --out-dir $(dirname {output})"

rule build_check_cross_feasibility_binary:
    threads: 1
    resources:
             cargo=1
    input:
        cargo="/home/gregor/Code/et/column_generation/src/bin/check_cross_feasibility.rs"
    output:
        OUTPUT_PREFIX + "/binaries/check_cross_feasibility"
    shell:
        "RUSTFLAGS='-C link-arg=-s -C target-cpu=native' cargo +nightly build -Z unstable-options -j 1 --profile=cluster --features='snakemake' --manifest-path /home/gregor/Code/et/column_generation/Cargo.toml --no-default-features --bin check_cross_feasibility --target-dir {OUTPUT_PREFIX}/target/ --out-dir $(dirname {output})"

rule build_remove_infeasible_binary:
    threads: 1
    resources:
             cargo=1
    input:
        cargo="/home/gregor/Code/et/column_generation/src/bin/remove_infeasible.rs"
    output:
        OUTPUT_PREFIX + "/binaries/remove_infeasible"
    shell:
        "RUSTFLAGS='-C link-arg=-s -C target-cpu=native' cargo +nightly build -Z unstable-options -j 1 --profile=cluster --features='snakemake' --manifest-path /home/gregor/Code/et/column_generation/Cargo.toml --no-default-features --bin remove_infeasible --target-dir {OUTPUT_PREFIX}/target/ --out-dir $(dirname {output})"