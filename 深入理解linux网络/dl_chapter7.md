**内核组件初始化**

## 注册关键字
&emsp;内核组件`__setup`宏，注册关键字和相关联的处理函数
```c
/*
 * Only for really core code.  See moduleparam.h for the normal way.
 *
 * Force the alignment so the compiler doesn't space elements of the
 * obs_kernel_param "array" too far apart in .init.setup.
 */
#define __setup_param(str, unique_id, fn, early)			\
	static const char __setup_str_##unique_id[] __initconst		\
		__aligned(1) = str; 					\
	static struct obs_kernel_param __setup_##unique_id		\
		__used __section(".init.setup")				\
		__aligned(__alignof__(struct obs_kernel_param))		\
		= { __setup_str_##unique_id, fn, early }

/*
 * NOTE: __setup functions return values:
 * @fn returns 1 (or non-zero) if the option argument is "handled"
 * and returns 0 if the option argument is "not handled".
 */
#define __setup(str, fn)						\
	__setup_param(str, fn, fn, 0)

```
> 上面宏转换后如下
```c 
struct obs_kernel_param {
	const char *str;
	int (*setup_func)(char *);
	int early;
};
static const char __setup_str_fn[] __initconst __aligned(1)  = str;
static struct obs_kernel_param __setup_fn __used_section(".init.setup") = struct obs_kernel_param{__setup_str_fn, fn, early}
// 被放到.init.setup section段 ,所以这些代码会在引导期间被执行
```
> 当内核在引导期间命令行参数遇到`str`就会执行`fn`函数，例如
```c
int __init netdev_boot_setup(char *str)
{
    // dosth
	return netdev_boot_setup_add(str, &map);
}

__setup("netdev=", netdev_boot_setup);
```
> 上面这段代码意思就是当命令行参数包含`netdev=`时候，kernel在启动的时候就会去执行`netdev_boot_setup`函数(string=后面的值会被当作参数传递给function)


## 引导期间相关网络设置
```c 
int __init netdev_boot_setup(char *str)
{
	int ints[5];
	struct ifmap map;       // ifmap 是输入配置值相关数据结构

	str = get_options(str, ARRAY_SIZE(ints), ints);
	if (!str || !*str)
		return 0;

	/* Save settings */
	memset(&map, 0, sizeof(map));
	if (ints[0] > 0)
		map.irq = ints[1];
	if (ints[0] > 1)
		map.base_addr = ints[2];
	if (ints[0] > 2)
		map.mem_start = ints[3];
	if (ints[0] > 3)
		map.mem_end = ints[4];

    // 解析str， 填充map配置

	/* Add new entry to the list */
	return netdev_boot_setup_add(str, &map);
}

__setup("netdev=", netdev_boot_setup);


// 处理的事情就是把引导配置及其配置添加到全局变量dev_boot_setup
static int netdev_boot_setup_add(char *name, struct ifmap *map)
{
	struct netdev_boot_setup *s;
	int i;

	s = dev_boot_setup;         // 全局变量
    // 由于同一个关键字可以在引导字符串里面出现多次，因此在引导期间限制最大可配置设备数据为NETDEV_BOOT_SETUP_MAX
    // 例如 linux ether=5,0x260,eth0 ether=15,0x300,eth1
	for (i = 0; i < NETDEV_BOOT_SETUP_MAX; i++) {
		if (s[i].name[0] == '\0' || s[i].name[0] == ' ') {
			memset(s[i].name, 0, sizeof(s[i].name));
			strlcpy(s[i].name, name, IFNAMSIZ);
			memcpy(&s[i].map, map, sizeof(s[i].map));
			break;
		}
	}
	return i >= NETDEV_BOOT_SETUP_MAX ? 0 : 1;
}
```

## 模块初始化代码
&emsp;kernel里面想要加载一个模块必须提供两个函数:
- `module_init`
> 模块被加载的时候会被执行,例如 `module_init(xxxx_init)`
- `module_exit`
> 模块被卸载的时候会被执行，例如 `module_exit(xxx_cleanup)`

&emsp; 一些相关的宏
- `__init`
> 引导期间初始化函数(针对到了引导阶段结束时，已经不再使用的函数)
- `__exit`
> 与`__init`配对,当相关的组件被关闭的时候，这个函数会被调用
- `core_initcall`
> 标记必须在引导阶段执行的函数
- `postcore_initcall`
- `arch_initcall`
- `subsys_initcall`
- `fs_initcall`
- `device_initcall`
- `late_initcall`
- `__initdata` 
> 引导期间已经初始化了的数据结构
- `__exitdata`
> 标记`__exitcall`函数所用的数据结构,如果标记`__exitcall`函数不再使用了, 则标记为`__exitdata`数据也不再使用.
