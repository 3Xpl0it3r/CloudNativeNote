## **常用数据结构**
#### 介绍
- struct sk_buff
>  一个sk_buff代表一个完整的封装好的packet. 它是一个动态数据结构，每经过一层都会添加一些对应层的数据

- struct net_device
> 在Linux内核里面每种网络设备都用net_device 来表示,(包括软件/硬件的配置信息)

- struct sock
> 用于存储套接字的网络信息

#### 套接字缓冲区: `sk_buff` 结构
&emsp;代表`已经`接收或者`正要`传输的的报头， 这个结构体非常庞大复杂，试图满足所有协议的需求。它的字段主要分为下面几类: `布局(Layout)`,`通用(General)`,`功能专用(Feature-specific)`,`管理函数(Management functions)`

&emsp;`sk_buff`每经过一个网络协议层，都需要开辟一些空间，在kernel里面提供了`skb_reserve`函数来执行
&emsp; 多个不同的网络分层都会使用这个结构,当该结构从一个分层传递到另外一个分层上的时候，其不同字段也会随之变化, 从L4传递给L3之前附加一个报头,L3传递给L2之前，也会加上自己的报头.(附加报头效率比把数据从一个分层拷贝到另外一个分层更有效率, 附加报头就需要开辟新的空间，通过skb_reserve来完成)


##### 布局字段
```c
struct sk_buff_head{
    struct sk_buff *next;       // sk_buff 里面都包含一个sk_buff_head的指针
    struct sk_buff *prev;

    __u32 qlen;             //代表元素的个数
    spinlock_t  lock;       // 防止并发访问
};
```

*sk_buff*
```c
struct sk_buff{
    struct sock *sk;            // 指向拥有此缓冲区的sock 数据结构, 当数据在本地产生或者由本地接收的时候就需要这个指针。
    unsigned int len;           // 缓冲区中数据区块的大小(这个长度包括主要的缓冲区数据以及一些fragment数据, 协议头也会算进去)

    unsigned int data_len ;     // 缓冲区数据区块的大小(但是只计算fragment 中数据大小)

    unsigned int mac_len;       // mac 头大小

    automic_t users;            // 引用计数器

    unsigned char *head ;
    unsigned char *end;
    unsigned char *data;
    unsigned char *tail;
    // 这些字段代表缓冲期的边界， data 和 tail 指向实际数据的开端和尾端
};

```

##### 通用字段
&emsp;通用的字段与特定的内核功能无关;
```c
struct sk_buff {
    struct net_device *dev;     //此字段描述一个网络设备 , 接受sk_buff ，dev就代表接收包的设备，  发包 dev 就是发包的设备

    struct net_device *input_dev;       // 已经被接收的封包所源自的设备 ,主要由流量控制所使用
    struct net_device *real_dev;        //  这个字段只针对虚拟设备有意义，代表虚拟设备所关联的真实设备

    union{...} h;       // h针对L4
    union{...} nh;      // nh针对 L3
    union{...} mac;         //mac针对L2

    struct dst_entry dst;               // 这个结构路由子系统使用
};
```

##### 功能专用字段
&emsp;Linux内核是模块化的，允许选择包含/省略什么，因此只有当内核编译为支持特定的功能，如防火墙/QOS,某些字段才会包含在sk_buff 数据结构里面
```
struct nf_bridge_info *nf_bridge    // 由netfilter 使用(network packet filtering debugging / bridged ip/arp packeet filter使用)
```



#### `net_device`结构
##### 标识符
```c
int ifindex             //独一无二的ID 当设备以dev_new_index注册时分派给每个设备
int iflink              // 这个字段主要由虚拟隧道设备使用
unsigned short dev_id   //  ipv6使用
```


##### 配置
&emsp;`net_device`数据结构存储了特定网络设备的所有的信息。 每个设备都有这样一个结构(无论是真实设备还是虚拟设备). 并且所有的设备都放在一个全局的变量里面`dev_base`里面管理..一些字段含义如下:
```c
struct net_device {
    unsigned int irq;                   // 用于与内核对话的中断编号, 这个值可以由多个设备共享。 驱动程序使用request_irq 函数分配此变量,并且使用free_irq 来释放.
    unsigned char dma;                  //dma 通道。为了从内核获取和释放DMA通道，文件kernel/dma.c定义了 `request_dma`和`free_dma` 函数

    /* 函数相关*/
    struct ethtool_ops *ethtool_ops;        // 指向一组函数指针（主要供ethool 来使用的)

    /*初始化/清理/销毁/关闭设备*/
    int (*init)(....);
    void (*uninit)(....);
    int (*open)(....);
    int (*stop)(....);

    /*状态相关的函数*/
    struct net_device_status* (*get_status)(...);               // 收集一些信息供用户态程序使用，例如ifconfig,ip等工具
    struct iw_statistics* (*get_wireless_status)(...);          //  针对无线设备状态统计

    /*网卡控制*/
    int (*do_ioctl)(...);

    /*驱动参数设置*/
    int (*set_config)(...);
};
```


#### 通用
```c 
atomic_t refcnt ;            引用计数器
int (*poll) (....)
struct list_head poll_list
int quota
int wright
//  上面三个字段和napi功能相关
struct list_head todo_list              网络设备注册和删除需要的
```
