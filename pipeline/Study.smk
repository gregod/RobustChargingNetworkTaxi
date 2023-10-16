rule studyn1: # 30 sites, virtual_*-1 , bat 1, tol 0,1,5
    priority: 100
    output: OUTPUT_PREFIX + "/study1"
    input:
         expand(OUTPUT_PREFIX + "/opt/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/opt_log",
                NUM_SITES=[30,50,70],time_bucket=["virtual_" + str(d) + "-" + str(ss) for d in range(0,2) for ss in [2,1]],BATTERY=["battery_1"],TOLERANCE=["00"])
    shell:
        "echo {input} > {output}"
rule studyn2: #, same as 1 stuff only with variable
    priority: 99
    output: OUTPUT_PREFIX + "/study2"
    input:
         expand(OUTPUT_PREFIX + "/opt/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/var_opt_log",
                NUM_SITES=[30,50,70],time_bucket=["virtual_" + str(d) + "-" + str(ss) for d in range(0,2) for ss in [2,1]],BATTERY=["battery_1"],TOLERANCE=["00"])
    shell:
        "echo {input} > {output}"
rule studyn3: #, same as 1 stuff only cross
    priority:98
    output: OUTPUT_PREFIX + "/study3"
    input:
         expand(OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/cross_opt_log",
                NUM_SITES=[30,50,70],time_bucket=["virtual_" + str(d) + "-" + str(ss) for d in range(0,2) for ss in [2,1]],BATTERY=["battery_1"],TOLERANCE=["00"])
    shell:
        "echo {input} > {output}"
rule studyn4: #, same as 1 stuff only cross
    priority:97
    output: OUTPUT_PREFIX + "/study4"
    input:
         expand(OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/var_cross_opt_log",
                NUM_SITES=[30,50,70],time_bucket=["virtual_" + str(d) + "-" + str(ss) for d in range(0,2) for ss in [2,1]],BATTERY=["battery_1"],TOLERANCE=["00"])
    shell:
        "echo {input} > {output}"


rule studyn5: # same as 1 but with tolerance
    priority: 100
    output: OUTPUT_PREFIX + "/study5"
    input:
         expand(OUTPUT_PREFIX + "/opt/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/opt_log",
                NUM_SITES=[30,50,70],time_bucket=["virtual_" + str(d) + "-" + str(ss) for d in range(0,2) for ss in [2,1]],BATTERY=["battery_1"],TOLERANCE=["00","01","05"])
    shell:
        "echo {input} > {output}"
rule studyn6: #, same as 1 stuff only with variable
    priority: 99
    output: OUTPUT_PREFIX + "/study6"
    input:
         expand(OUTPUT_PREFIX + "/opt/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/var_opt_log",
                NUM_SITES=[30,50,70],time_bucket=["virtual_" + str(d) + "-" + str(ss) for d in range(0,2) for ss in [2,1]],BATTERY=["battery_1"],TOLERANCE=["00","01","05"])
    shell:
        "echo {input} > {output}"
rule studyn7: #, same as 1 stuff only cross
    priority:98
    output: OUTPUT_PREFIX + "/study7"
    input:
         expand(OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/cross_opt_log",
                NUM_SITES=[30,50,70],time_bucket=["virtual_" + str(d) + "-" + str(ss) for d in range(0,2) for ss in [2,1]],BATTERY=["battery_1"],TOLERANCE=["00","01","05"])
    shell:
        "echo {input} > {output}"

