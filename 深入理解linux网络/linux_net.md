
** 网络设备中断**
&emsp;系统初始化-网络初始化里面注册软中断,在PCI注册里里面(启动网卡注册硬中断)
## 系统初始化
&emsp;概况
![img]()
&emsp;Linux启动(kernel初始化)
```c
asmlinkage void __init start_kernel(void)  // kernel启动
{
    // 引导期间一些选项(主要是内核配置参数)
    parse_early_param();                            // parse kernel command line
    parse_args("Booting kernel", static_command_line, __start___param, __stop___param - __start___param, -1, -1, &unknown_bootoption); 
    init_IRQ();                 // 硬中断初始化, 依赖平台的, 具体代码 arch/<arch>/kernel/irqinit.c
    softirq_init();             // 软中断初始化
    init_timers();              //  定时器
    ftrace_init();              // ftrace 初始化
    rest_init();                // kenel初始化函数,其他系统的初始化(kernel子系统,内置的驱动程序)
}

// 软中断初始化
void __init softirq_init(void)
{
	int cpu;
	for_each_possible_cpu(cpu) {
		int i;
        // 每种CPU都有两种tasklet，分别对应高优先级任务，和低优先级的tasklet
		per_cpu(tasklet_vec, cpu).tail = &per_cpu(tasklet_vec, cpu).head;
		per_cpu(tasklet_hi_vec, cpu).tail = &per_cpu(tasklet_hi_vec, cpu).head;
		for (i = 0; i < NR_SOFTIRQS; i++)   
			INIT_LIST_HEAD(&per_cpu(softirq_work_list[i], cpu));
	}

    // 注册tasklet 中断及其对应的handler
    // 对应低优先级的微任务
	open_softirq(TASKLET_SOFTIRQ, tasklet_action); 
    // 对应高优先级的微任务
	open_softirq(HI_SOFTIRQ, tasklet_hi_action);   
}

```
&emsp;初始化函数
```c
static noinline void __init_refok rest_init(void)
{
    // kernel 初始化
    kernel_thread(kernel_init, NULL, CLONE_FS | CLONE_SIGHAND);
}
static int __ref kernel_init(void *unused) 
{
    kernel_init_freeable();
    // 运行 init_filename 这个程序(可以通过init=xxxx来引导期间指定另外一个不同的程序来当作os第一个进程)
    // init  是os上面第一个进程
    static int run_init_process(const char *init_filename);       
    // 加载一些默认模块
    load_default_modules();                                       
}

static noinline void __init kernel_init_freeable(void)
{
    do_basic_setup();
}
// 到这一步机器本身基本上已经初始化完成了(cpu子系统，内存已经就绪),但是还没有任何驱动
static void __init do_basic_setup(void)
{
    shmem_init();                       // 共享内存相关系统的初始化
    driver_init();                      // 驱动初始化
    init_irq_proc();                    // 初始化中断
    // 初始化一些子系统(do_initcalls 遍历__initcall_start到__initcall_end 连表上所有函数,并执行,网络初始化也是在这一步)

    do_initcalls();                     
}

static void __init do_initcalls(void)
{
    /*
    这些是通过core_install, subsyscall等宏标记函数
    可以通过宏展开可以看到这些被宏标记的代码会被放到.initcall section段
    do_initcalls() 就是遍历 init_call_start 到 init_call_end之间的函数，来执行
    #define subsyscall(fn) module_init(fn)
    // driver initialization entry point
    #define module_init(x)	__initcall(x);
    #define __initcall(fn)  device_initcall(fn)
    #define device_initcall(fn)		__define_initcall(fn, 6)
    #define __define_initcall(fn, id) \
    static initcall_t __initcall_##fn##id __used \
    __attribute__((__section__(".initcall" #id ".init"))) = fn; \
    LTO_REFERENCE_INITCALL(__initcall_##fn##id)
    */
	int level;
	for (level = 0; level < ARRAY_SIZE(initcall_levels) - 1; level++)
		do_initcall_level(level);
}
```

## 设备硬中断注册
&emsp;任何一个设备添加到系统里面都需要完成初始化工作(NIC也不例外):
- 硬件初始化:       驱动和总线层完成(IO地址信息,IRQ注册)
- 软件初始化:       协议层初始化(支持那些协议)
- 功能初始化:       具体协议/子系统自己功能相关的配置/选项初始化


####  硬中断注册
&emsp;中断事件都会运行一个函数(irq handler),而`irq handler`必须按照设备所需要裁剪.例如当设备驱动注册一个NIC的时候,它会请求并且分派一个IRQ. 两个注册/删除IRQ的函数定义下`kernel/irq/manage.c`,并且由`arch/<arch>/kernel/irq.c`中对应的函数改写(这个是当网卡启动的时候会执行硬中断注册)
```c 
// 注册 => 实际就是将一个irqaction 对象放入到全局irq_desc 向量里面()
typedef struct {
	unsigned int status;		/* IRQ status */
	hw_irq_controller *handler;
	struct irqaction *action;	/* IRQ action list */
	unsigned int depth;		/* nested irq disables */
	spinlock_t lock;
} ____cacheline_aligned irq_desc_t;
irq_desc_t irq_desc[NR_IRQS] __cacheline_aligned =
	{ [0 ... NR_IRQS-1] = { 0, &no_irq_type, NULL, 0, SPIN_LOCK_UNLOCKED}};
// 1
int request_irq(unsigned int irq, irq_handler_t handler, unsigned long irqflags, const char *devname, void *dev_id)
{
    /*
    request_irq 会注册一个handler(这个irq必须是有效的,并且还没有分配给其他的人)
    */
    struct irqaction *action = (struct irqaction *)kmalloc(sizeof(struct irqaction), GFP_KERNEL);   // 申请一个irq(一个struct irqaction就代表一个软中断)
    action->handler = handler; action->name = devname; ..... ;# 填充irqaction 结构体
    
    // 调用setup_irq(irq, action); 来真正去注册IRQ
    return  setup_irq(irq, action);
}
// 2.
int setup_irq(unsigned int irq, struct irqaction * new)
{
    // 将new插入全局irq_desc里面
	irq_desc_t *desc = irq_desc + irq; // desc = irq_desc[irq]

	p = &desc->action;
	*p = new;
	register_irq_proc(irq);
	return 0;
}
// 例如百兆网卡IRQ注册(在e100_up里面被调用)
// eg err = request_irq(adapter->pdev->irq, e100_intr;, irq_flags, netdev->name, netdev);// 触发中断的时候会执行e1000_intr 这个函数


// 删除
void free_irq(unsigned_int irq, void *dev_id)
{
    /*
    给定的设备由dev_id 标识,此函数会删除例程,而且如果没有其他设备注册在该IRQ上，就关闭该IRQ
    */
}
```
> 当内核接收到中断时候，会使用IRQ编号找到该驱动程序对应的handler,然后执行这个handler.
&emsp;IRQ和handler在内核里面可能是如下表现形式
```c 
irq_desc[IRQ_0] = irqaction00->irqaction01->.....->irqaction0N;
irq_desc[IRQ_1] = irqaction10->irqaction11->.....->irqaction1N;
....
```

#### IRQ handler的映射
&emsp;映射通过`irqaction`来定义，全局irq向量`irq_desc`, 处理中断的内核函数是体系依赖的.
```c 
struct irqaction {
    //设备驱动提供的函数，当设备触发一个硬中断，内核就会调用这个handler
	void (*handler)(int, void *, struct pt_regs *);
	unsigned long flags;
	unsigned long mask;
    // 设备名称
	const char *name;
    // 设备ID(net_device 指针)
	void *dev_id;
	struct irqaction *next;
};
```


#### 网络设备初始化
&emsp;前面我们已经知道Linux里面通过`subsys_initcall`宏来初始化各个子系统
```c 
static int __init net_dev_init(void)
{

	INIT_LIST_HEAD(&ptype_all);              // 初始化ptype_all 双链表
	for (i = 0; i < PTYPE_HASH_SIZE; i++)   // #define PTYPE_HASH_SIZE 16
		INIT_LIST_HEAD(&ptype_base[i]);     // 初始化ptype_base hash连表

	if (register_pernet_subsys(&netdev_net_ops)) goto out;  // 注册network namespace 子系统
        /*
        static struct pernet_operations __net_initdata netdev_net_ops = {
        .init = netdev_init,            //  当创建一个namespace的时候会执行netdev_init函数
        .exit = netdev_exit,            //  当删除/销毁一个namespace的时候会执行netdev_exit 函数
        };
        */

    // todo dosth
    软中断注册
    sysfs注册
    dst_init 初始化(用于协议无关目的缓存)
    ptype_base 初始化 
    OFFLINE_SAMPLE
    dev_cpu_callback 

    net_device_init();
}
// subsys_initcall(net_dev_init);

void __init net_device_init(void)       // 网络设备初始化
{
	/* Devices supporting the new probing API */
	network_probe();            // 
    ...
}
static struct net_probe pci_probes[] __initdata = {{NUlL, 0}}
static void __init network_probe(void)
{
	struct net_probe *p = pci_probes;
	while (p->probe != NULL) {
		p->status = p->probe();
		p++;
	}
}
```



## PCI层

&emsp;PCI设备由驱动程序`pci_driver`结构定义
```c 
struct pci_driver 
{
    char *name;                                                                     // 驱动名称
    const struct pci_device_id  *id_table ;                                         //  ID向量，内核用于把一些设备关联到这个驱动程序
    // 当pci层发现它正在寻找驱动的设备的id与前面提到的id_table匹配，就会调用这个函数
    int (*probe) (struct pci_dev *dev, const struct pci_device_id *d)                       // 这个函数主要执行开启硬件,分配net_device结构,初始化注册新设备(这个函数也会分配所需的数据结构)
    void (*remove) (struct pci_dev *dev)                                                    // 当设备被删除的时候执行这个函数
    int (*suspend) (struct pci_dev *dev, pm_message_t state)                         // 挂起模式执行这个函数
    int (*resume) (struct pci_dev *dev)                                             // 恢复时候执行这个函数
    int (*enable_wake) (struct pci_dev *dev, u32 state, int enable)                 // 电源管理事件

    struct pci_dynids dynids                                //动态ID
}
```


### PCI NIC设备驱动注册
&emsp;pci独一无二的识别方式通过一些参数组合(厂商,模型),这些参数由内核定义在`pci_device_id`类型的数据结构里面
```c
/**
 * struct pci_device_id - PCI device ID structure
 * @vendor:		Vendor ID to match (or PCI_ANY_ID)      厂商
 * @device:		Device ID to match (or PCI_ANY_ID)      设备id          通常vendor+device就可以识别一个设备
 */
struct pci_device_id {
	__u32 vendor, device;		//  vendor + device 就可以识别一个设备
	__u32 subvendor, subdevice;	            
	__u32 class, class_mask;	// 代表该设备所属的类, 例如 NETWORK 就是一个类
	kernel_ulong_t driver_data;	 //这个不属于pci_device_id 一部分， 它是驱动程序使用的一个私有参数
	__u32 override_only;
};
```
&emsp;每个设备驱动程序都会把一个`pci_device_id`实例注册到内核里面,这个实例向量列出了它所能处理的设备ID
&emsp;PCI 注册和删除两个函数
- `pci_register_driver`
&emsp;注册
```c 
// 借助pci_driver里面的id_table 向量，kernel就可以知道它可以处理哪些设备 
#define pci_register_driver(driver)	 __pci_register_driver(driver, THIS_MODULE, KBUILD_MODNAME)
```

- `pci_unregister_driver`
&emsp;删除
```c 
void pci_unregister_driver(struct pci_driver *dev);
```
&emsp;Linux探测机制主要有两种:
- 静态
> 给定一个PCI ID 内核就能根据id_table向量查询出正确的PCI驱动程序----静态探测
- 动态
> 用户手动配置ID, 一般在调试模式下使用


### 总结(伪代码)
```c 
const struct pci_device_id _xxx_table[] = {
    { vendor, device_id1, subvendor, subdevice, class, class_mask, driver_data, },
    { vendor, device_id2, subvendor, subdevice, class, class_mask, driver_data, },
}
// probe 函数 (用__devinit 宏来标记)
static int __devinit xxx_probe(struct pci_dev *pdev, const struct pci_device_id *ent){ dosth ;}
// remove 函数 (用__devexit宏来标记)
static void __devexit xxx_remove(struct pci_dev *pdev) { dosth ;}

// suspend 函数 和 resume  函数
static int xxx_suspend(struct pci_dev *pdev, u32 state){ dosth;}
static int xxx_resume(struct pci_dev *pdev)

// driver
static struct pci_driver xxx_driver = {
    .name = "设备名称",
    .id_table = xxx_id_table,
    .probe = xxxx_probe,
    .remove = __devexit_p(xxx_remove),      // 用__devexit_p宏标注了下
    .suspend = xxx_suspend,
    .resume = xxx_resume,
}


// PCI设备注册
static int __init xxx_init_module(void)
{
	return pci_register_driver(&xxx_driver);
}
// 卸载设备 
static voud __exit xxx_cleanup_module(void)
{
    pci_unregister_driver(&xxx_driver)
}

module_init(xxx_init_module)        // 模块初始化的调用xxx_init_module
module_exit(xxx_cleanup_module)     // 设备删除的时候调用这个函数
```
