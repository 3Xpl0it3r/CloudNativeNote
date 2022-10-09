**设备注册和初始化** 
> page:146
&emsp;NIC在可用之前,与之相关联的`net_device`数据结构必须先被初始化,然后添加到内核设备数据库

## 设备注册
&emsp;网络设备注册会有两种情况:
- 加载NIC 设备驱动程序(适用于总线模型)
> 1. 如果NIC设备驱动内建到内核里面,那么它会在引导期间初始化
> 2. 如果模块方式加载,那么它会在运行期间初始化(每当发生初始化的时候,驱动程序会控制所有的NIC都会被注册)
- 热插拔设备
> 用户把NIC设备插入到设备里面,内核会通知其驱动程序,而驱动程序再去注册这个设备(一般驱动会提前加载好)
## 设备删除
&emsp;卸载设备也会有2种情况:
- 卸载NIC 网卡(针对模块加载)
> 当管理员卸载NIC驱动时候,所有关联的NIC必须被除名
- 删除可热插拔设备
> 当用户从系统里面删除可热插拔NIC的时候,则网络设备就会被除名

## 分配`net_device`结构
&emsp;网络设备利用`net_device`来定义.一般在kernel代码里面通常称为`dev`, 由函数`alloc_netdev`来分配
```c 
/**
 *	@sizeof_priv:	size of private data to allocate space for
 *	@name:		device name format string
 *	@setup:		callback to initialize device
*/
#define alloc_netdev(sizeof_priv, name, name_assign_type, setup) \
	alloc_netdev_mqs(sizeof_priv, name, name_assign_type, setup, 1, 1)

/**
 *	Allocates a struct net_device with private data area for driver use
 *	and performs basic initialization.  Also allocates subquue structs
 *	for each queue on the device.
 */
struct net_device *alloc_netdev_mqs(int sizeof_priv, const char *name,
		void (*setup)(struct net_device *),
		unsigned int txqs, unsigned int rxqs)
{
	struct net_device *dev;             //  分配的设备地址
	size_t alloc_size;
	struct net_device *p;


	alloc_size = sizeof(struct net_device);
	p = kzalloc(alloc_size, GFP_KERNEL);

	dev->pcpu_refcnt = alloc_percpu(int);       // 当前设备被多少CPU引用.
	if (!dev->pcpu_refcnt)
		goto free_p;

	if (dev_addr_init(dev))                 // 初始化dev_addr 连表
		goto free_pcpu;

	dev_mc_init(dev);                       // 设备硬件初始化
	dev_uc_init(dev);                       // 单播地址初始化

	dev_net_set(dev, &init_net);


    // 相关连表初始化
	INIT_LIST_HEAD(&dev->napi_list);        
	INIT_LIST_HEAD(&dev->unreg_list);
	INIT_LIST_HEAD(&dev->link_watch_list);
	INIT_LIST_HEAD(&dev->upper_dev_list);
	setup(dev);                             // 调用setup函数来对device做一些设置

	dev->num_tx_queues = txqs;              // 设置tx队列数量
	dev->real_num_tx_queues = txqs;
	if (netif_alloc_netdev_queues(dev))            // 分配队列
		goto free_all;

	strcpy(dev->name, name);                        // 设置设备的名称 , name%d 形式
	dev->group = INIT_NETDEV_GROUP;                 
	if (!dev->ethtool_ops) 
		dev->ethtool_ops = &default_ethtool_ops;        // 设置default_ethtool_ops  //主要设置ethtool client的回调函数
	return dev;
}
```
&emsp;kernel也会提供一组`alloc_netdev`包装的函数例如:
|网络设备|包裹函数名称|包裹函数定义|
|----|----|----|
|以太网|`alloc_etherdev`|`return alloc_netdev(sizeof_priv, "eth%d", ether_setup)`|
|FDDI|`alloc_fddidev`|`return alloc_netdev(sizeof_priv, "fddi%d", fddi_setup)`|
|令牌环|`alloc_trdev`|`return alloc_netdev(sizeof_priv, "tr%d", tr_setup)`|

## 注册/卸载
&emsp;注册流程: 使用`alloc_etherdev`(对`alloc_netdev`做了封装)分配`net_device`结构,`ether_setup`函数会对设备做一些设置,然后`register_netdev`函数为设备注册.
```c 
dev = alloc_etherdev(sizeof(driver_private_structure))
        -> alloc_etherdev(sizeof_priv, "eth%d", ether_setup) 
            ->  dev = kmalloc(sizeof(net_device) + sizeof_priv + padding)
            ->  ether_setup(dev)
            - > strcpy(dev->name, "eth%d")
netdev_boot_setup_check(dev)                                // 用来检查加载内核的时候是否提供了引导期间的参数
    -> ....
register_netdev(dev)
    -> register_netdevice(dev)                      // register_netdevice 会把设备插入到设备数据库里面
```

&emsp;卸载时候会调用`unregister_netdevice`和`free_netdev`(`dev->destructor`函数).设备驱动会释放设备所使用的任何资源(IRQ,内存mapping等)


## 设备初始化
&emsp;初始化分为下面三类:
- 设备驱动程序
> IRQ,I/O内存以及I/O端口
- 设备类型
> 对同一类型系列的所有设备通用字段初始化.由`xxx_setup`函数负责.例如Ethernet设备使用的是`ether_setup`
- 各种功能
> 强制和可选功能耶必须初始化
&emsp; 设备类型初始化属于设备驱动初始化的一部分(即xxx_setup是由xxx_probe来调用,因此驱动程序由机会改写默认设备类型的初始化)
*xxx_setup 和 xxx_probe 初始化的net_device函数指针*
- xxx_setup:
> change_mtu, set_mac_address, rebuild_header, hard_header
- 设备驱动程序的探测函数
> open, stop, hard_start_xmit, tx_timeout, get_stats, do_ioctl

## 设备驱动程序初始化
&emsp;设备程序驱动初始化由`xxxx_probe`函数负责. 有些驱动程序可以处理不同的设备模型.所以相同的参数可以根据设备模型和能力初始化为不同的值。


## 设备类型初始化
&emsp;针对网络设备类型而言,`xxx_setup`函数可以针对`net_device`结构中相同类型的所有设备通用的字段做初始化(包括参数和函数指针),(ether_setup函数是通过alloc_xxxdev函数传递给alloc_ntdev)。eg:`ether_setup`函数如下
```c 
void ether_setup(struct net_device *dev)
{
	dev->header_ops		= &eth_header_ops;
	dev->type		= ARPHRD_ETHER;
	dev->hard_header_len 	= ETH_HLEN;
	dev->mtu		= ETH_DATA_LEN;
	dev->addr_len		= ETH_ALEN;
	dev->tx_queue_len	= 1000;	/* Ethernet wants good queues */
	dev->flags		= IFF_BROADCAST|IFF_MULTICAST;
	dev->priv_flags		|= IFF_TX_SKB_SHARING;
	memset(dev->broadcast, 0xFF, ETH_ALEN);
}
```
> 使用通用的wrapper函数，以及xxx_setup 是比较常见的方式, 但是也有特例:
- 有些类型设备定义了自己的setup函数，但是没有提供wrapper函数
- xxx_setup 可能会被不属于指定种类的设备所用.


## `net_device` 结构的组织
&emsp;在使用`net_device`应该注意下面一些东西:
- `alloc_netdev`在分配`net_device`时候,会把驱动程序私有数据区域大小传进去.
- `dev_base` 和 `net_device`中的next 指针指向了 `net_device`结构的开头,而不是分配区域的开头.(开头补空白空间的大小则存储在dev->padded, 使得kernel可以在合适的时间释放整个内存区域)
&emsp;`net_device`数据结构被插入在一个全局列表里面(dev_base)和两个hash表里面(dev_index_head, dev_name_head)
- `dev_base`
> 包含所有`net_device`实例的全局列表,可以让kernel轻易的浏览设备, 因为每个驱动对私有数据都有自己的定义，net_device 结构全局链表链接的元素可能大小不一样
- `dev_name_head`
> hash表, 以设备名称为索引. 例如通过ioctl 接口应用某项配置变更时。 老一代配置工具通过ioctl接口与内核童话，通常会用设备名称引用设备
- `dev_index_head`
>  hash表, 以设备id `dev->ifindex`为索引. 对`net_device`结构做交叉引用时,通常会存储设备ID或者指向`net_device`结构的指针. 新一代的配置工具ip 通过netlink 套接字与kernel交互, 通常就是用设备的ID  引用设备


## `net_device`查询
&emsp;常见的设备查询方式通过设备名称或者设备的ID查询.这两种查询实现分别由`dev_get_by_name`和`dev_get_by_index`负责. ------>  利用 两张 hash table查询
&emsp;另外也可以通过设备类型或者 mac 地址搜索`net_device`，这种查询基于 `dev_base` 全局链表查询
> 这三张表均有`dev_base_lock` 保护


## 设备的状态
&emsp;`net_device`定义当前状态的字段如下:
- flags:
> 用于存储各种标识的位于. 多数标识都代表设备的能力。然而IFF_UP 代表设备是开启还是关闭.
- reg_state
> 注册状态
- state
> 和它的队列规则有关的设备状态

## 队列规则状态
&emsp;每个设备都会被分派一种队列规则,流量控制以此实现QoS机制.`net_device`的`state`字段是流量控制所用结构字段之一.
- `__LINK_STATE_START`
> 设备开启, 此标识可以用`netif_running`检查
- `--LINK_STATE_PRESENT`
> 设备存在
- `__LINK_STATE_NOCARRIER`
> 没有载波
- `__LINK_STATE_XOFF`
- `__LINK_STATE_SHED`
- `__LINK_STATE_RX_SCHED`
>  上面三个标识管理设备入口和出口流量使用



## 注册的状态
&emsp;设备和网络协议之间的注册状态存储在`net_device`结构里面的`reg_state` 字段里面. 这个字段的值`NETREG_XXX` 值都定义在`include/linux/netdevice.h`
- `NETREG_UNINITIALIZED` 
>  定义0，当net_device 数据结构以及分配，并且内容以及全部清0， 此值代表就是dev->reg_state中的0
- `NETREG_REGISTERING`
> net_device 已经添加到更早的`net_device`结构的组织,但是还没有在/sys文件系统里面添加一个项目 
- `NETREG_REGISTERED`
> 以及完成注册了
- `NETREG_UNREGISTERING`
- `NETREG_UNREGISTERED`
- `NETREG_RELEASED`

## 设备的注册和删除
&emsp; 网络设备通过`register_netdev`  和`unregister_netdev` 在内核里面注册和删除(这两个函数只是简单的wrapper ,负责加锁). 这两个函数最终会调用`register_netdevice`和`unregister_netdevice`.
- 状态改变在`NETREG_UNINITIALIZED`和`NETREG_UNREGISTERED`之间是由`netdev_run_todo`来处理的
- 设备的注册和删除都是由`netdev_run_todo`来完成的.


## 切割操作:`netdev_run_todo`
&emsp;`register_netdevice`负责一部分的注册，然后在调用`netdev_run_todo`来完成，(netdev_run_todo在rtnl_unlock里面调用)
```c 
int register_netdev(struct net_device *dev)
{
	int err;

	rtnl_lock();                            
	err = register_netdevice(dev);
	rtnl_unlock();                          
	return err;
}
int register_netdevice(struct net_device *dev)
{
    // ----------- 开始注册
	struct net *net = dev_net(dev);
    ... dosth()
	dev->reg_state = NETREG_REGISTERED;             //注册状态改变

    // ---- net_device添加到todo 列表里面

	linkwatch_init_dev(dev);            // 添加到dev_base/ dev_index_head/ dev_name_head

}
```
> `rtnl_unlock` 不仅会释放该锁,也会调用`netdev_run_todo`。`netdev_run_todo` 会浏览`net_todo_list`数组,然后完成其全部的`net_device`实例注册.(任何时刻只有一个CPU可以执行netdev_run_todo, 串行通过net_todo_run_mutex互斥强制实施)

&emsp;`unregister_netdevice`负责卸载.
```c 
static void efx_unregister_netdev(struct efx_nic *efx)
{

    // ...  清理资源
	rtnl_lock();
	unregister_netdevice(efx->net_dev);
	efx->state = STATE_UNINIT;
	rtnl_unlock();
}
static inline void unregister_netdevice(struct net_device *dev)
{
	unregister_netdevice_queue(dev, NULL);
}
void unregister_netdevice_queue(struct net_device *dev, struct list_head *head)
{
	if (head) {
		list_move_tail(&dev->unreg_list, head);
	} else {
		rollback_registered(dev);
		net_set_todo(dev);
	}
}
```

&emsp;`rtnl_unlock`函数
```c 
void rtnl_unlock(void)
{
	/* This fellow will unlock it for us. */
	netdev_run_todo();
}

void netdev_run_todo(void)
{
    // dosth
}
```

## 设备注册
&emsp; 设备注册模型里面：注册将`net_device` 加入到全局列表和hash表里面,`net_device`里面参数初始化,通知其他的组件(`register_netdev` -> `register_netdevice`)

### `register_netdevice`函数

```c 
int register_netdevice(struct net_device *dev)
{
	int ret;
	struct net *net = dev_net(dev);



    // 获取验证devname
	ret = dev_get_valid_name(net, dev, dev->name);


    // 设置dev 私有变量
    dev->ifindex = dev_new_index(net);
    dev->iflink = dev->ifindex;

	dev->hw_features |= NETIF_F_SOFT_FEATURES;
	dev->features |= NETIF_F_SOFT_FEATURES;
	dev->wanted_features = dev->features & dev->hw_features;


	dev->vlan_features |= NETIF_F_HIGHDMA;
	dev->hw_enc_features |= NETIF_F_SG;

    // 链式通知
	ret = call_netdevice_notifiers(NETDEV_POST_INIT, dev);


    // 创建sys条目
	ret = netdev_register_kobject(dev);
    // 更改设备状态
	dev->reg_state = NETREG_REGISTERED;

	__netdev_update_features(dev);

    // 设置状态位
	set_bit(__LINK_STATE_PRESENT, &dev->state);

    
    // carrier or dormant
	linkwatch_init_dev(dev);

    // 调度队列相关(qdisc, tx_queue, ingress_queue)
	dev_init_scheduler(dev);
	dev_hold(dev);                      // cpu 引用计数器+1
    // 将网卡添加到全局链表里面(dev_base)和两张全局hash表里面(dev_name_head, dev_index_head)
	list_netdevice(dev);

	if (dev->addr_assign_type == NET_ADDR_PERM)
		memcpy(dev->perm_addr, dev->dev_addr, dev->addr_len);

    // 完成初始化
	ret = call_netdevice_notifiers(NETDEV_REGISTER, dev);
    dev->rtnl_link_state == RTNL_LINK_INITIALIZED)
    rtmsg_ifinfo(RTM_NEWLINK, dev, ~0U);
}
```
> 疑问点: `todo_list`作用是什么? 在register里面没有发现`set_netdev_todo`,但是在`netdev_run_todo`却有消费的行为


## 设备取消注册
&emsp;`unregister_netdevice`也是接收一个参数
```c 
static inline void unregister_netdevice(struct net_device *dev)
{
	unregister_netdevice_queue(dev, NULL);
}

void unregister_netdevice_queue(struct net_device *dev, struct list_head *head)
{   // head here is NULL
    rollback_registered(dev);           // 将dev放置到dev->unreg_list 链表里面
    /* Finish processing unregister after unlock */
    net_set_todo(dev);                  // 执行net_set_todo(dev)  将当前的dev->todo_list放到 全局net_todo_list里面
}
```
> 疑问点: `dev->todo_list` 里面元素是在哪添加进去的?

## 开启设备
&emsp;


## 关闭设备


## 用户态空间配置设备及其相关的信息
### Ethtool
&emsp;`ethtool`数据结构中有一个指向类型为`ethtool_ops` 的VFT指针, `ethtool_ops`结构是一组函数指针, 可用于读取和初始化`net_device`结构上的许多参数,或者用于触发一种行为
&emsp;并不是所有的设备启动都支持，但是哪些支持这些驱动程序并不一定支持所有的函数。一般而言`dev->ethtool_ops` 的初始化在`probe`函数里面完成


