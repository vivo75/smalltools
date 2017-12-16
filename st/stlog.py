#!python
# Copyright 2017 Francesco Riosa
# Distributed under the terms of the GNU General Public License v2

import signal
import sys
# This block ensures that ^C interrupts are handled quietly.
try:

	def exithandler(signum, _frame):
		signal.signal(signal.SIGINT, signal.SIG_IGN)
		signal.signal(signal.SIGTERM, signal.SIG_IGN)
		sys.exit(128 + signum)

	signal.signal(signal.SIGINT, exithandler)
	signal.signal(signal.SIGTERM, exithandler)

except KeyboardInterrupt:
	sys.exit(128 + signal.SIGINT)

import os
import time
import subprocess
import argparse


class writer:
	f = None
	ondisk = False

	def __init__(self, filename, append):
		mode = "w"
		if filename is not None:
			self.ondisk = True
			if append:
				mode = "a"
			self.f = open(filename, mode)

	def out(self, msg):
		print(msg, end='')
		if self.ondisk:
			self.f.write(msg)

def main(argv):
	
	selfname = os.path.basename(argv[0])

	parser = argparse.ArgumentParser(description='mix std{out,err}')

	parser.add_argument('-t', '--tee', dest='filename',
						help='duplicate output to file')
	parser.add_argument('-a', '--append', dest='append', action='store_true',
						help='append (rather than create) output file')
	parser.add_argument('command', nargs=argparse.REMAINDER)

	args = parser.parse_args(argv[1:])

	w = writer(args.filename, args.append)

	w.out("{}:exec:'{}'\n".format(selfname, args.command))

	if w.ondisk:
		w.out("{}:log:'{}'\n".format(selfname, os.path.realpath(args.filename)))

	prog = subprocess.Popen(
		args.command,
		stdout = subprocess.PIPE,
		stderr = subprocess.PIPE,
		bufsize = 0,
		universal_newlines = False,
		errors = 'backslashreplace',
		)

	while prog.poll() is None or so or se:
		so = prog.stdout.readline()
		se = prog.stderr.readline()
		if so != "":
			w.out("[1,{0:.9f}] {1}".format(time.monotonic(), so))
		if se != "":
			w.out("[2,{0:.9f}] {1}".format(time.monotonic(), se))

	w.out("{}:rc:{}\n".format(selfname, prog.returncode))

if __name__ == '__main__':
	sys.exit(main(sys.argv))
