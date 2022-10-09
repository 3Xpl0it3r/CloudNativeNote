**PIC层和NIC**
&emsp;涉及数据结构如下:
- `pic_device_id`:
> 设备标识符(这个是PCI标准所定义的ID)
- `pci_dev`
> 每个PCI设备都会被分派一个`pci_dev`实例(如果每个网络设备都会被分配一个`net_device`一样，这个是kernel使用，以引用一个PCI设备)
- `pci_driver`
> 定义PCI层和设备驱动程序之间接口,这个接口主要由函数指针组成,所有的PCI设备都会使用这个结构

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


## PCI NIC设备驱动注册
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
```c 
/*
    借助pci_driver里面的id_table 向量，kernel就可以知道它可以处理哪些设备 
*/
#define pci_register_driver(driver)		\
	__pci_register_driver(driver, THIS_MODULE, KBUILD_MODNAME)

```
- `pci_unregister_driver`
```c 
void pci_unregister_driver(struct pci_driver *dev);
```
&emsp;Linux探测机制主要有两种:
- 静态
> 给定一个PCI ID 内核就能根据id_table向量查询出正确的PCI驱动程序----静态探测
- 动态
> 用户手动配置ID, 一般在调试模式下使用


## 总结(伪代码)
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
