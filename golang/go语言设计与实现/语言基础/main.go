package main

type Duck interface {
	Quack()
}

type Cat struct {
	Name string
}

func (c *Cat) Quack() {}

func main() {
	var c Duck = &Cat{Name: "dasdasa"}
	c.Quack()
	c.(*Cat).Quack()

}
