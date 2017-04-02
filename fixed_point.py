import subprocess
import sys

f = open("./test.c", "r")
cur_file = bytes(f.read(), "utf8")
f.close()

p = subprocess.run(["target/debug/quine"],
                   input=cur_file,
                   stdout=subprocess.PIPE)
new_file = p.stdout

count = 0
while cur_file != new_file:
  print("count {}".format(count),file=sys.stderr)
  cur_file = new_file
  p = subprocess.run(["target/debug/quine"],
                    input=cur_file,
                    stdout=subprocess.PIPE)
  new_file = p.stdout
  count += 1

print(cur_file.decode('utf-8'))
