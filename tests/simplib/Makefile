test: test-run
	./test-run

test-run: test.c
	gcc -o $@ test.c -L ../../target/debug/ -l ffizz_tests_simplib -l dl -l pthread
