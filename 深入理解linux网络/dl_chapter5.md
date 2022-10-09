** 网络设备中断**
&emsp;系统初始化-网络初始化里面注册软中断,在PCI注册里里面(启动网卡注册硬中断)
## 系统初始化
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

&emsp;中断类型是设备告诉驱动程序可以做哪些事情.一般NIC有如下几种中断类型:
- 接收一个帧
> 最常见/标准情况
- 传输失败
> `exponential binary backoff`时候，由ethernet设备产生(硬件产生)
- DMA传输完成了
> 给定一个帧,当帧加载到NIC的内存,并且准备在此媒介上开始传输时,驱动程序就会把持有该帧的缓冲区给释放掉.使用同步传输的时候(没DMA当帧到了NIC，驱动程序就会立刻知道,当使用了DMA 时候,也就是异步传输, 驱动必须等待NIC设备发出一个明确的中断才能开始处理)
- 设备有足够的内存处理新的请求
> 当出口队列没有足够空间存储一个最大帧(1536字节),NIC 驱动程序会停止出口队列而关闭传输,当内存可用时候队列又会再次开启.

#### 中断共享
&emsp;IRQ是有限的资源,Linux做法是允许多个设备共享IRQ.(通常情况下每个设备会针对该IRQ将自己的handler注册到kernel里面,再由kernel启用注册了同一个IRQ的所有的handler，然后由各自的handler来过滤是不是错误的调用)
&emsp;注册好比"分配一个IRQ_n给我，并且把fn作为它的handler"


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
