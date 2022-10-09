**系统初始化**
## 网络设备初始化
&emsp;驱动程序可以作为模块加载到内核里面,也可以作为内核的静态组件。

#### 系统初始化概论
&emsp;当内核引导的时候，会执行`start_kernel`对一些子系统做初始化
&emsp; 一个网络设备可以用就必须被内核认可，并且关联正确的驱动程序。驱动程序把驱动设备所需要的信息都存储到私有设备里面, 然后把其他需要设备的内核组件交互。

- 硬件初始化
&emsp;由驱动程序和通用总线层合作完成(PCI/USB). 
- 软件初始化
&emsp;设备在使用之前，依赖所开启和配置的网络协议为何而定，用户需要提供一些配置
- 功能初始化
&emsp; 相关option设置

#### NIC 初始化的基本目标
- IRQ 线
> 在NIC 必须被分配一个IRQ,(虚拟设备不需要分派IRQ)
- I/O 端口和内存注册
> 直接映射(虚拟地址即kernel地址)


#### 设备与内核之间的交互
&emsp;几乎所有的设备(包括NIC)都采用下面两种方式之一与内核交互:
- 轮询(Polling)
> 由内核端驱动(内核会定期检查设备的状态)------延迟
- 中断
> 由设备驱动(设备触发一个硬中断) -------------中断的代价是昂贵的


#### 硬件中断
&emsp;每个中断事件都会运行一个函数(中断处理 interrupt handler),.当设备驱动注册一个NIC 的时候，会请求并且分配一个IRQ. 然后调用两个函数注册或者删除中断handler。
- `int request_irq(unsigned int irq, void (*handler)(int, void*, struct pt_regs*), unsigned long irqflags, const char* devname, void *dev_id)` 
> 注册中断及其handler

- `void free_irq(unsigned int irq, void *dev_id)`
> 给定设备，函数此设备绑定的中断及其handler(如果没有其他的设备使用这个 IRQ)(需要注意共享IRQ的情况)


&emsp;当内核接收到中断信号时，会使用IRQ编号找到该驱动的handler，然后执行这个handler。(kernel会把所有的中断编号及其对应的handler存储在一个全局的表里面)(irq_desc)


#### 中断类型
&emsp;NIC的中断类型/处理的事件
- 接收一帧
> 最常见/标准的情况
- 传输失败
> 硬件产生，高层网络层无感知，高层网络层通过其他的方式感知(例如ACK等)
- DMA传输已经完成
> 给定一个帧传输，当帧DMA 到内存准备传输时候，驱动程序会将持有该帧的缓冲区释放(当帧上传到NIC,驱动程序就立马知道了,但是使用DMA的时候,由于是异步的，因此驱动程序必须等待NIC发出明确的中断)
- 设备有足够的内存处理新传输
> 当队列没有足够的空间保存一个最大尺寸的帧的时候，NIC设备驱动程序会停滞出口队列，并且关闭传输，当内存可用的时候，该队列又会再次开启该队列


#### 中断共享
&emsp;一个IRQ可能对应多个设备(handler),kernel会启用所有的IRQ的handler(类似广播),由每个设备的handler来自行判断自己是不是需要执行handler.

#### IRQ-handler mapping
```c
struct irqaction
{
    void(*handler) (int irq, void *dev_id, struct pt_regs *reg); // irq:  产生此通知的IRQ编号, dev_id: 设备表识符号. regs: 存储处理器寄存器相关信息(当前进程一些信息)

    unsigned long flags ;       // 一组表识 SA_SHIRQ: 设备驱动可以共享IRQ， SA_SAMPLE_RANDOM:设备自身变为随机事件来源  SA_INTERRUPT: handler运行在本地处理器上，并且中断处于关闭状态

    void *dev_id;   //  此设备相关了的net_device 数据结构的指针

    struct irqaction *next;     //所有共享用一个IRQ编号的设备会用此指针链接成一个列表

    const char *name;   //设备名称
};
```

#### 设备处理层初始化: net_dev_init
&emsp;网络代码初始化代码如下(`net/core/dev.c`, 函数:`net_dev_init`)
```c
static int __init net_dev_init(void)
{
    // 当内核编译支持/proc 文件系统时，一些文件会通过dev_proc_init 和dev_mcast_init 添加到/proc立马
     __init dev_proc_init(void);
    // netdev_sysfs_init 向sysfs 注册为net类.---会自动创建/sys/class/net/目录 ,每个注册的网络设备都有一个子目录
    int register_pernet_subsys(struct pernet_operations *ops);
    // net_random_init 为每个CPU都初始化种子向量，用于net_random函数产生随机数 

    // ptype_base初始化，用于分离入口流量的多路合并传输 
    for (i = 0; i < PTYPE_HASH_SIZE; i++){INIT_LIST_HEAD(&ptype_base[i]);}
    // 针对每个CPU初始化softnet_data 数据结构
    for_each_possible_cpu(i){
        struct softnet_data  *sd = &per_cpu(softnet_data, i);
        // do sth to config sd
    }
    // 两个网络软中断(softirq)所使用的对应的各个cpu的数据结构被初始化
    open_softirq(NET_TX_SOFTIRQ, net_tx_action);
    open_softirq(NET_RX_SOFTIRQ, net_rx_action);
    // 把一个回调处理handler注册到发出CPU热插件事件的通知信息的通知链(dev_cpu_callback)
    hotcpu_notifier(dev_cpu_callback, 0);
    // 与协议无关的目的缓存(protocol-independent destionation cache, DST) dst_init 初始化
    dst_init();
}

subsys_initcall(net_dev_init);
```

## PCI层和网络接口
&emsp;涉及到数据结构:
- `pci_device_id`
> 设备标识符。这个是PCI标准所定义的ID(不是Linux定义的)
- `pci_dev`
> 每个PCI设备都会被分配一个PCI_dev的实例, 如果每个网络设备都会被分派一个`net_device`一样
- `pci_driver`
> 定义PCI层和设备驱动程序之间的接口. 这个结构主要由函数指针组成, 所有的PCI设备都会用到这个结构

&emsp;`pci_driver`结构体说明
```c
struct pci_driver
{
    char *name ;        // 驱动名称
    const struct pci_device_id *id_table;       // 这个是ID向量,内核用于把一些设备关联到这个驱动程序
    int (*probe) (struct pci_dev *dev, const struct pci_device_id *id);             // 当pci层发现它正在搜寻的驱动程序设备ID与前面的 ID匹配上了，就会调用这个程序. 此函数中驱动程序也会分配正确工作所需要的数据结构
    void (*remove)(struct pci_dev *dev)  ;                  // 当驱动程序内核里面除名的时候，或者可热插拔被删除时候，PCI层就会调用这个函数. 这个函数和probe 函数对应，用来清理申请的数据结构和状态.(网络设备使用这个函数来释放已经分配的i/o端口和i/o内存)

    // pci设备挂起和恢复操作使用到的函数
    int(*subspend) (struct pci_dev *dev, pm_message_t state);
    int(*resume) (struct pci_dev *dev);
};
```

#### PCI NIC 设备驱动程序注册
&emsp;PCI设备独一无二的识别方式通过一些参数组合。包括开发商以及一些模型，这些数据由内核存储在`struct pci_device_id`结构里面.
```c
struct pci_device_id
{
    unsigned int vendor, device;            // 通常这两个字段就可以识别设备
    unsigned int subvendor, subdevie;
    unsigned int class, class_mask;             //设备所属的类
    unsigned long driver_data;              //  这个是驱动程序所使用的一个私有的参数
};
```

&emsp;pci设备注册和卸载函数: `pci_register_drive` 和 `pci_unregister_driver`
&emsp; 每个PCI设备会把

## 组件初始化的内核基础框架
