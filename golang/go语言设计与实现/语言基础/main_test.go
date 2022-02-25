package main

import "testing"

func BenchmarkDirectCallPointer(b *testing.B) {
	c := &Cat{Name: "dasas"}
	for n := 0; n < b.N; n++ {
		c.Quack()
	}
}

func BenchmarkDynamicDispatch(b *testing.B) {
	c := Duck(&Cat{Name: "dssaa"})
	for n := 0; n < b.N; n++ {
		c.Quack()
	}
}
func BenchmarkDynamicDispatchV1(b *testing.B) {
	c := Duck(&Cat{Name: "dssaa"})
	for n := 0; n < b.N; n++ {
		c.(*Cat).Quack()
	}
}

func BenchmarkDirectCallStruct(b *testing.B) {
	c := Cat{Name: "dasas"}
	for n := 0; n < b.N; n++ {
		c.Quack()
	}
}
