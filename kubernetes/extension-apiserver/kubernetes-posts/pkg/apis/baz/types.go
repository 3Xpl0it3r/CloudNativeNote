package baz

// 在这个里面没有定义资源的marshal 和 protobuf的转换协议，因为我们不需要给用户可以定义内部资源的权限，内部的版本不是给用户用的
// 用户能定义的只有外部资源
// 你也许会注意到我们在定义spec字段的时候和外部版本有点不一样，这个主要演示在不同的版本之间我们是如何做转换的

import metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"

// +genclient
// +k8s:deepcopy-gen:interfaces=k8s.io/apimachinery/pkg/runtime.Object

type Foo struct {
	metav1.TypeMeta
	metav1.ObjectMeta

	Spec FooSpec
}

type FooSpec struct {
	// +k8s:conversion-gen=false
	Bar []FooBar
}

type FooBar struct {
	Name string
}

// +k8s:deepcopy-gen:interfaces=k8s.io/apimachinery/pkg/runtime.Object

type FooList struct {
	metav1.TypeMeta
	metav1.ListMeta

	Items []Foo
}

// +genclient
// +genclient:nonNamespaced
// +k8s:deepcopy-gen:interfaces=k8s.io/apimachinery/pkg/runtime.Object

type Bar struct {
	metav1.TypeMeta
	metav1.ObjectMeta

	Spec BarSpec
}

type BarSpec struct {
	// cost is the cost of one instance of this topping.
	Description string
}

// +genclient:nonNamespaced
// +k8s:deepcopy-gen:interfaces=k8s.io/apimachinery/pkg/runtime.Object

type BarList struct {
	metav1.TypeMeta
	metav1.ListMeta

	Items []Bar
}
