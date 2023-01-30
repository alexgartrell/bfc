This is a bad, "optimizing" compiler for brainfuck

Programs are all from [http://brainfuck.org/](http://brainfuck.org/)
without any changes to the
[licensing](https://creativecommons.org/licenses/by-sa/4.0/legalcode)
implied.

```
cargo run < programs/fib.b | gcc -nostdlib -x assembler -o fib - && ./fib
```
