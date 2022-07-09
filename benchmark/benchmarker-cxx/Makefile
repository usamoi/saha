build-release:
	mkdir -p ./target/release
	clang++ ./main.cpp -std=c++20 -o ./target/release/main -O3 -march=native

build-debug:
	mkdir -p ./target/debug
	clang++ ./main.cpp -std=c++20 -o ./target/debug/main -O1 -march=native

clean:
	rm -rf ./target

run: build-release
	./target/release/main --path ../../datasets
