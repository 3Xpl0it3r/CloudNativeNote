package v1alpha1

import "k8s.io/apimachinery/pkg/runtime"

func addDefaultingFuncs(scheme *runtime.Scheme) error {
	return nil
}

func SetDefaults_FooSpec(obj *FooSpec) {
	if len(obj.Bar) == 0 {
		obj.Bar = []string{"bar1", "bar2", "bar3"}
	}
}
