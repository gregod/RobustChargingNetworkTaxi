
OUTPUT_PREFIX = "work"
rule tmp:
    input: OUTPUT_PREFIX + "/opt/{NUM_SITES}/{time_bucket}/{FILENAME}"
    output: OUTPUT_PREFIX + "/opt/{NUM_SITES}/{time_bucket}/battery_1/{FILENAME}"
    shell:
         "mv {input} {output}"