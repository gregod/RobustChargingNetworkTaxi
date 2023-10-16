#!/usr/bin/env python3
"""
Snakemake SLURM scheduler script

Usage:
snakemake -j 1000 --debug --immediate-submit --cluster 'snakemake_sbatch.py
{dependencies}'
"""
from subprocess import (
    PIPE,
    Popen,
    STDOUT,
)
import sys
from time import sleep

from plumbum import ProcessExecutionError
from plumbum.cmd import (
    grep,
    squeue,
)
from snakemake.utils import read_job_properties

def sec2time(time):

   seconds = time % 60
   minutes = (time / 60) % 60
   hours = (time / 3600) % 24
   days = time / 86400

   if days < 0 or  hours < 0 or minutes < 0 or seconds < 0:
     time_str = "INVALID"
   elif days:
     return u"%ld-%2.2ld:%2.2ld:%2.2ld" % (days, hours, minutes, seconds)
   else:
     return u"%2.2ld:%2.2ld:%2.2ld" % (hours, minutes, seconds)


class SnakemakeSbatchScheduler():
    '''Class for scheduling snakemake jobs with SLURM

    All parameters are automatically produced by snakemake

    Parameters
    ----------
    jobscript: str
    Path to a snakemake jobscript
    dependencies: List[str]
    List of SLURM job ids
    '''

    def __init__(self, jobscript, dependencies=None):
        self.jobscript = jobscript
        self.dependencies = dependencies
        job_properties = read_job_properties(jobscript)
        errprint(job_properties)
        
        if(str(job_properties['type'])=='group'):
            self.jobname = str(job_properties['groupid'])
        else:
            self.jobname = str(job_properties['rule'])
        self.threads = str( min([8, int(job_properties.get('threads', '1'))]) )
        self.mem = str(job_properties['resources'].get('mem_mb', '3500'))
        self.time = sec2time(job_properties['resources'].get('runtime', 60 * 5))
        self.command = self.construct_command()

    def construct_command(self):
        '''Construct the sbatch command from the jobscript'''
        cmd = ['sbatch']

        # Construct sbatch command
        if self.dependencies:
            cmd.append("--dependency")
            cmd.append(','.join(['afterok:%s' % d for d in self.dependencies]))

        cmd += ['--job-name', self.jobname,
                '--cpus-per-task', self.threads,
                '--ntasks', '1',
        #		'--clusters','serial',
        #		'--partition','serial_std',
                '--time',self.time,
                '--output','work/logs/%j.%N.out.txt',
                '--error','work/logs/%j.%N.err.txt',
        #		'--mail-user','g.godbersen@tum.de',
        #		'--mail-type','FAIL',
                '--parsable',
                '--mem', self.mem,
                self.jobscript]

        return cmd

    def print_summary(self):
        '''Print a summary of the submitted job to stderr'''
        errprint('Submit job with parameters:')
        errprint('name: ' + self.jobname)
        errprint('threads: ' + self.threads)
        errprint('mem(mb): ' + self.mem)
        errprint('sbatch command: ' + ' '.join(self.command))

    def has_remaining_dependencies(self):
        '''Return True if the job has dependencies'''
        for dependency in self.dependencies:
            try:
                (squeue | grep[dependency])()
                return True
            except ProcessExecutionError:
                pass
        return False

    def submit(self):
        '''Submit job to SLURM'''
        if self.jobname == 'all':
            # If snakemake is passed the immediate-submit parameter, its main
            # process terminates as soon as all jobs are submitted. This
            # masks further updates on job completions/failures. To avoid this
            # we wait until all jobs are complete before submitting 'all' for
            # scheduling.
            while self.has_remaining_dependencies():
                sleep(10)

        self.print_summary()
        sbatch_stdout = Popen(self.command, stdout=PIPE,encoding='utf8',
                              stderr=STDOUT).communicate()[0]

        # Snakemake expects the job's id to be sent to stdout by the
        # scheduler
        print("%i" % int(sbatch_stdout.split(";")[0]), file=sys.stdout)


def errprint(x):
    '''Print to stderr'''
    print(x, file=sys.stderr)


if __name__ == "__main__":
    # Snakemake passes a list of dependencies followed by the jobscript to the
    # scheduler.
    jobscript = sys.argv[-1]

    if len(sys.argv) > 2:
        dependencies = sys.argv[1:-1]
    else:
        dependencies = None

    sbatch = SnakemakeSbatchScheduler(jobscript, dependencies)
    sbatch.submit()
