**传输和接受**

&emsp;一般情况下:传输只是把帧往外传送而已,而接受是指帧传进来.

** 中断和网络驱动程序 **

## 决策和流量方向
&emsp;封包经过的网络协议栈所经过的路径,随着封包是接受的,传输的,还是转发的都会变得不同.
&emsp;虚拟设备(例如lo)一般会倾向于使用网络协议栈里面的快捷方式(这些虚拟设备只是软件). lo与任何硬件都无关,但是绑定接口则和一块或者多块网络卡间接相关.(例如有些虚拟设备可以免除硬件上某些限制，例如mtu)， 因此可以提高网络性能.

## 接受到帧时通知驱动程序

### 轮询
&emsp;内核可以不中断的持续检查设备是不是有packet进来(例如kernel可以持续的轮询设备上的某一个寄存器,或者设置一个定时器，定时去检查设备寄存器),但是这种做法会浪费很多系统的资源(如果os和设备可以使用其他的技术例如中断，那么轮询就会很少被采用，但是在某些情况下轮询却是最优选择)
### 中断
&emsp;当特定的事件发生的时候,驱动会代表kernel让设备产生一个硬中断,这个时候kernel会暂停其他的工作，然后执行驱动注册的中断对应的handler. 
&emsp;当事件是接收到一个帧时,处理函数就会把该帧排入到队列某个地方,然后通知内核,(中断在低流量负载情况下最佳选择,但是在高流量下就无法良好的运行了,因为产生中中断的代价是昂贵的，每一帧产生一个中断，会让CPU在处理中断这件事情上浪费很多时间)

&emsp;负责接收帧的代码分为两部分: 首先驱动程序会把帧拷贝到kernel可以访问的输入队列里面. 然后内核在做处理, 通常是把帧传给一个相关协议(例如IP).(第一部分会在中断环境中执行,而且是可抢占第二部分的执行. 也就说接收输入帧并将其拷贝到队列的代码比实际处理帧的代码优先级要高)

&emsp;在高流量负载情况下,中断代码会持续抢占正在处理的代码.(到某一个时间点,输入队列会满, 但是由于应该让帧退出队列并予以处理的代码的优先级低而没有机会执行. 结果系统就崩溃). 新的帧无法排入队列，因为没有新的空间，而旧的帧无法得到处理(因为没有CPU可供其使用)---这种情况称为`接收-活锁`(`receive-livelock`)

&emsp;总之这种技术优点是帧的接收及其处理之间的延迟很短,但是高负载情况下无法很好的工作.(多数网络驱动都使用中断)


## 在中断期间处理多帧
&emsp;很多驱动处理方式如下: 通知kernel 中断,并且中断handler也执行了，然后协议处理函数会持续下载帧，然后将他们放入到输入队列里面(直到帧达到最大数目为止),有可能一直做下去,直到队列清空为止(但是驱动程序应该良好的，因此驱动程序必须与其他的子系统共享CPU，与其他的设备共享IRQ)
&emsp;内存占用也必须要限制下，每个设备的内存数量是有限的，因此能存储的帧也是有限的(如果驱动程序不能以实时方式来处理帧，那么缓冲区很快会被填满,而新的帧会被丢弃).如果承受负载的设备与之处理进来的帧，直到队列为空，那么饿死的情形就会发生在其他的设备身上.
&emsp; 这种技术的其他变种, 驱动程序可能只关闭某个设备中断事件(功能), 该设备的输入队列中的有一些帧，然后把轮询驱动程序的队列的任务交给一个kernel处理的函数,不再是把所有的中断都关闭，之后令驱动程序为kernel把帧排入队列以便处理。而这真是为什么Linux采用新的接口----NAPI原因

## 定时器驱动的中断事件
&emsp;驱动程序会指示设备定期产生一个中断事件(不再是异步方式通知驱动程序有关帧的接收),然后处理函数会检查自从上次中断事件以来是否有任何帧的达到,然后一次处理所有的帧.
&emsp;根据定时器的粒度不同，设备所接收的帧会体现不同等级的延迟. 设备驱动程序能用的粒度取决于设备要提供什么,因为定时器是在硬件里面实现的.(目前只有少数的设备提供这种能力)，只有少数的硬件设备提供这种能力，当然驱动可以关闭中断，然后该用kernel的定时器，来模拟这种能力。然而驱动没有硬件的支持，处理定期时CPU能话费的资源也不像设备那么多，所以驱动没有办法让定时器的调度过于频繁，最后绕到了轮询的方法了，

## 组合
&emsp;在低负载的情况下，纯中断模型可以保证低延迟，但是高负载情况下，纯中断模型就不行了.
&emsp;定时器驱动的中断事件在低负载下可能引入过多的延迟，而浪费太多的CPU时间,但是高负载情况下，可以大量减少CPU用量并且能解决接收-活锁的问题. 
&emsp;好的组合是在低负载情况下使用中断技术，在高负载情况下切换到定时器驱动中断事件


## 中断处理函数
&emsp;每当CPU接收一个IRQ的时候，就会调用与该中断相互关联的handler,这种关联由编号识别。在执行handler期间(即内核处于中断环境interrupt context)中,服务于该中断事件的CPU会被关闭其中断功能(也就是说如果一个CPU忙于服务于一个中断事件，就不能服务于其他的事件, 无论是相同类型或者不同类型的中断事件),并且CPU也不能服务于其他的函数,而且不能被抢占。(cpu完全服务于该中断函数)
&emsp;在最简单的情况下，这些是一个中断事件流程如下:
- 设备产生一个中断事件(硬件通知内核)
- 如果kernel没有为另外一个中断服务(而且如果中断功能没有因为其他原因而被关闭)就会看到该通知信息
- 内核关闭本地CPU中断功能，然后执行接收中断对应事件关联的handler
- kernel会离开该中断处理函数，然后重启本地CPU的中断功能
> 中断handler是非抢占的,而且是非可再进入的(当一个函数无法被另一个自身的调用而中断，这个函数就被定义为非可在进入). 就中断处理函数而言，也就是说其执行中断功能会关闭. 这种设计用来降低竞争的可能性。因为CPU能做的事情很有限，kernel做非可抢占设计以及等待被CPU服务的进程，就会对性能有潜在的严重的影响。
&emsp;因此中断函数做的功能尽可能的少，尽可能的快.中断所处理的函数在中断期间所需的处理量以来事件的类型(例如键盘可能只需在每次有一个按键被按下的时候传送一个中断事件,而处理此事件秩序与很少的能力，而且最多没秒执行几次。其他的事件，处理一个中断事件所需要的动作并非琐碎事情，而且执行需要话费很多CPU时间,例如z网络设备就相当的工作量，必须分配一个缓冲区(sk_buff) 把接收到的数据copy进去，对缓冲区结构(protocol)内的一些参数做初始化，以告知较高层次协议处理函数来自驱动程序数据是什么种类的，等等)

&emsp;尽管一个中断事件所触发的动作需要用到很多CPU时间,通常此动作的内容多数可以等待。中断事件可以先对CPU抢占，这是因为如果os 让硬件等太久可能会丢失数据.
&emsp;另外一方便，如果内核态或者用户态的程序必须被延迟或者抢占，则没有数据丢失的风险。 因此现代中断处理函数就分为上下两部分，上半部分是释放CPU之前必须执行的事情(一般用来保存数据), 下半部分可以从容的做完一切事情
```c 
static int kvm_cpu_get_extint(struct kvm_vcpu *v)
{
	if (kvm_cpu_has_extint(v))
		return kvm_pic_read_irq(v->kvm); /* PIC */
	return -1;
}

```
&emsp;下半部分可以定义成执行特定函数的异步请求。一般而言，当想要执行函数只是，不用请求任何东西，直接执行就行了。当一个中断事件达到的时候，很多事情要做，不想立刻中断. 因此可以把很多工作打包成一个函数，成为下部分函数

&emsp;与简单模型相比，下面模型允许kernel把中断功能关闭的时间大部分减少
1. 设备发出中断IRQ给CPU，通知CPU有中断事件.
2. CPU 会执行相关的上半部分，关闭后续的中断，直到此中断的上半部分函数被执行完
3. 一般而言上半部分函数会执行下面工作
- 把kennel稍后处理该中断事件的所有信息保存在ram的某个地方
- 在某处标记一个标识(或者使用另外一种内核机制来触发某事),以确保kernel会知道该中断事件.而且会用处理函数所保存的数据以完成该事件的处理
- 在终止前，会重启开启本地CPU的中断事件通知信息功能.
4. 稍后某一刻，当内核不再做一些紧迫的事情，就会检查该中断函数所设置的标识(指出有要求处理的数据存在),然后调用相关的下半部分函数。内核也会清理该标识，稍微当中断处理函数再次设置标识时，就会在认出来.


## 下半部分解决方案
&emsp;kernel提供了各种不同机制以实现各种下半部以及一般延期性质的工作，这些机制的主要差别在于:
- 运行环境
> kernel把来自于kernel代码和用户态代码的中断事件区别看待。当下半部分函数所执行的函数可以休眠的时候，
- 并发和上锁
> 当一种机制可以利用`SMP`时，就会涉及到如何强制串行化以及锁机制如何影响伸缩性


## 中断
- `in_interrupt`
> 如果CPU正在服务于一个硬中断或者软中断或者抢占功能是关闭的，in_interrupt 就返回TRUE
- `in_softirq`
>  如果cpu正服务于一个软中断的时候，那么它就返回true。
- `in_irq`
> 如果CPU正在服务于一个硬中断，in_irq 就返回true
- `softirq_pending`
> 如果CPU至少有一个软IRQ在未决中(也就在调度准备执行),就返回true。
- `local_softirq_pending`
> 如果本地CPU至少有一个软irq在未决中，返回true
- `__raise_softirq_ifrqoff`
- `raise_softirq_irqoff`
- `raise_softirq`
- `__local_bh_enable`
- `local_bh_enable`
- `local_bh_disable`
- `local_bh_disable`
- `local_irq_disable`
- `local_irq_enable`
- `local_irq_save`
> 会先把本地的cpu的中断状态保存起来，然后在予以关闭
- `local_irq_restore`
- `spin_lock_bh`
- `psin_unlock_bh:WTFPL`


##  抢占功能
&emsp;在分时系统里面，kernel总是能按照其意愿抢占用户的进程，但是kernel本身一般是不可抢占的，也就是说一旦它开始运行了，就不能被中断了，除非它自己准备好放弃控制权. 非可抢占内核有时会阻碍以及准备好可以执行的高级优先进程,因为内核正在为一个低优先级别进程执行系统调用.为了指出实时扩展功能及其其他的原因,linux2.5里面实现了完全抢占(preemptible),有了实时抢占.系统调用和其他内核任务就可以被其他较高的优先级的kernel任何给抢占掉.


## 下半部函数
&emsp;下半部基础架构需要解决如下问题:
- 把下半部分分类成适当的类型
- 注册下半部类型及其处理函数之间的关联关系
- 为下半部函数调度，已准备执行
- 通知内核有已调度的BH存在

#### 内核2.2版本的下半部函数
&emsp;2.2版本内核模型把下部分函数分为一大群类型,差别在于内核何时已何种频度予以检查和执行(这里面最感兴趣是NET_BH)
&emsp;每种下半部分类型都可以通过`init_bh`而关联上一个函数处理函数,例如网络代码把`NET_BH`下半部类型初始化为`net_dev_init`中的`net_bh`处理函数
```c 
__initfunc(int net_dev_init(void))
{
    ....
    init_bh(NET_BH  , net_bh);
    ...
}
```
&emsp;BH的删除函数`remove_bh`
&emsp;每当中断handler想要触发下半部函数执行时,就必须用`mark_bh`设置相对应的标识.这个函数做的事情很有限,在全局位图`bh_active` 中设置一个位,
```c 
external inline void mark_bh(int nr)
{
    set_bit(nr, &bh_active);
}
```
> 每次网络设备驱动程序成功接收一个帧后, 就会调用`netif_rx`来通知kernel。kernel会把新接收的帧排进入口队列里面`backlog`(会被所有CPU共享,然后标记`NET_BH`下半部函数标识)
```c 
skb_queue_tail(&backlog, skb):
mark_bh(NET_BH);
return;
```
&emsp;在几个函数的运行中，kernel会检查是否有任何下半部程序进度调度准备执行.如果有任何下半部程序处于等待中,kernel就会执行`do_bottom_half`来执行下半部程序
- `do_IRQ`:
> 每当kernel接收到IRQ，就会调用`do_IRQ`来执行相关联的处理handler. 因为中断handler会使得很多下半部程序进入调度(应该如何做才能使得他们延迟小于 do_IRQ结束时立刻启用另外一个函数的时间). 因此定时器中断事件,以`HZ`频率为到期单位，代表两个`do_bottom_half`连续执行间隔的时间上限.
- 中断和异常事件返回

- `schedule`:
> 此函数决定CPU接下来要执行什么,它会检查是否有任何下半部函数处于未决中,然后给予高于其他任务的优先级
```c 
asmlinkage void schedule(void)
{
    if (bh_mask & bh_active)
        goto handle_bh;
handle_bh_back:
    ....
handle_bh:
    do_bottom_half();
    goto handle_bh_back;
    ...
}
```
&emsp;`do_bottom_half`使用的函数`run_bottom_half`会执行没有执行的handler.
```c 
active = get_active_bhs();
clear_active_bhs(active);
bh =bh_base();
do {
    if (active & 1)
        (*bh)();
    bh ++;
    active >>= 1;
} while (active)
```
&emsp;这些处于没有执行的handler的调用顺序取决于位图里面相关联标识的位置,以及扫描这些标识的方向(`get_active_bhs`返回, 因此下半部执行不是谁先来谁先得的服务的方式进行的.), 另外网络下半部可能会执行很长的时间,那些不幸的最后才从队列退出的下半部handler就会经历很长的延时.(在早期的版本里面kernel禁止下半部handler并发执行,任何时候只有一个handler可以运行. 无论CPU有多少个)


## kernel2.4 BH引入 SoftIRQ
&emsp;2.4以后引入了softirq(可以看成是下半部函数的多线程版本)(softirq可以同时运行，而且相同的softirq 可以在不同的CPU上运行,并发唯一的限制就是cpu 上同一时间只能有一个IRQ运行), softirq模型:
```c 
enum
{
	HI_SOFTIRQ=0,
	TIMER_SOFTIRQ,
    //网络子系统相关的软中断
	NET_TX_SOFTIRQ,         // net_tx_action
	NET_RX_SOFTIRQ,         // net_rx_action
	BLOCK_SOFTIRQ,
	BLOCK_IOPOLL_SOFTIRQ,
	TASKLET_SOFTIRQ,
	SCHED_SOFTIRQ,
	HRTIMER_SOFTIRQ,
	RCU_SOFTIRQ,    /* Preferable RCU should always be the last softirq */

	NR_SOFTIRQS
};
```
&emsp;在旧的模型里面`XXX_BH`的下半部依然是可以用的，但是这些旧类型以及全部重新实现了,以`HI_SOFTIRQ`类型(表示优先级高于其他的IRQ类型)的软IRQ运行.

&emsp;和旧的下半部一样,软`IRQ`执行时,中断功能是开启的,因此任何时候都可以挂起,以处理新进来的中断请求,又在该CPU上运行,这样可以大幅度减少所需的上锁量. 每种软`IRQ`的新请求又在该CPU上运行，这样可以大幅度减少所需的上锁量. 每个软件`IRQ`类型都维护了一个类型为`softnet_data`的数据结构数组，而每个CPU都有一个`softnet_data`类型的数据结构,用来存储当前软`IRQ`的状态信息。 因为每一种类型的软`IRQ`的不同实例可以同时在不同的CPU上运行，因此软`IRQ`所执行的函数还是必须锁住其他共享的数据结构,用来避免竞争情况发生.

&emsp;软`IRQ`函数是以`open_softirq`函数注册的，而和`init_bh`不一样的是，此函数会接收一个而外的参数，当需要的时,可以传一些输入数据给函数处理函数，然而软`IRQ`都还没有使用到这个额外的参数,`open_softirq`只是吧输入参数拷贝到`kernel/softirq.c`声明的全局数组里面`softirq_vec`里面，而此数组存的类型和处理函数之间关联:
```c 
static struct softirq_action softirq_vec[NR_SOFTIRQS] __cacheline_aligned_in_smp;
void open_softirq(int nr, void (*action)(struct softirq_action *))
{
	softirq_vec[nr].action = action;
}
```
&emsp;软`IRQ`可以通过下面函数在本地CPU上进入调度而准备执行:
- `__raise_softirq_ifrqoff`
> 此函数在2.2版本里面是`mark_bh`的配对函数，只是设置与要执行的软`IRQ`相关联的标识,稍后在检查此标识时, 相关联的处理函数就会被调用
- `raise_softirq_irqoff`
> 这是内涵`__cpu_raise_softirq`的warped函数，如果此函数不是从硬件或者软中断环境里面调用，而且抢占功能没有被关闭，就会另外再为ksoftirqd线程调度。如果此函数是从软中断里面调用，调用此线程就不是必要的，因为`do_softirq`一定会被调用
- `raise_softirq`
> `raise_softirq_irqoff` 的函数，而且在执行时候，中断功能会被关闭

&emsp;irq 开发初期所用的模版.此模型非常类似2.2模型，而且会调用函数`do_softirq`，该函数是2.2里面`do_bottom_half`，如果至少有一个软irq以及进入调度了,`do_softirq`就会被调用.
```c 
asmlinkage  void schedule(void)
{
    // dosth like manage for here has no any lock
    if (softirq_active(this_cpu) && softirq_mask(this_cpu))
        goto handle_softirq;

handle_softirq_back:
    ... dosth

handle_softirq:
    do_softirq();
    goto handle_softirq_back;
}
```
&emsp;软`IRQ`模型必须以每个CPU为基准检查标识,因为每个CPU都有自己未决的软IRQ位图.
&emsp;`do_softirq` 的实现非常类型于其2.2版本中的配对函数`do_bottom_half`。内核也会在一些相同的点调用此函数，但是并不完全相同. 主要引入了一个各个CPU的内核线程`ksoftirqd`
&emsp;`do_softirq` 会被调用的主要地方在于:
- `do_IRQ`:
> 定义在各个体系结构文件`arch/arch-name/kernel.irq.c`中`do_IRQ`架构如下:
```c 
fastcall unsigned int do_IRQ(struct pt_regs *regs)
{
    irq_enter();
    ....
    // 使用以及注册处理函数处理irq编号
    ...
    irq_exit();
    return 1;
}
```

- `local_bh_enable`
> 当一个CPU上再次重启开启irq的时候,调用`do_softirq`就可以处理未决的req了.
- kernel的`softirqd_cpun`
> 为了组织软irq垄断所有的CPU(这种情况在高负载的网络上毕竟容易发生，以为`nNET_TX_SOFTIRq`和`net_tx_softirq`中断事件的优先级要高于用户进程),因此引进了新的一组per-cpu线程. 这些线程名称是`ksoftirqd_cpu0`和`ksoftirqd_cpu1`...等

&emsp;另外一个调用`do_softirq`的有趣的地方是在netif_rx_ni里面 内建至内核的流量产生器也会调用do_softirq



## 微任务
&emsp;微任务是函数，可以延迟某一个中断事件或者其他的任务，使其晚一点执行.微任务由中断任务发出，但是内核其他部分也会用到微任务。

&emsp;`HI_SOFTIRQ`属于高优先级的微任务，而`TASKLET_SOFTIRQ`是用于实现比较低优先级的微任务。每次发出延迟执行请求后，`tasklet_struct` 结构的一个实例就会被排入一由`HI_SOFTIRQ`所处理的列表，或者另一个由`TASKLET_SOFTIRQ`所处理的列表

&emsp;由于软`IRQ`是由每个CPU各自处理的，每个CPU都有两份未决的`tasklet_struct`列表,一份和`HI_SOFTIRQ`关联，一份和`TASKLET_SOFTIRQ` 关联

&emsp;`tasklet_struct`结构定义如下:
```c 
struct tasklet_struct
{
    /*
    用于关联到一个CPU的未决结构链接起来的指针, 新元素由函数tasklet_hi_schedule 和 tasklet_schedule添加到头部
    */
	struct tasklet_struct *next; 
    /*
    位图标识, 其可能的取值由tasklet_state_xx 枚举在include/linux/interrpt.h

    */
	unsigned long state;

    /*
    有些情况下，可能必须暂时关闭而后重启开启微任务，这是由计数器完成的，0 表示微任务被关闭(因此不可执行),而非0值意味着微任务已经开启
    其值由 tasklet[_hi]_enble 和 tasklet[_hi]_diable 函数来递增和递减
    */
	atomic_t count;
    /*
    func是要执行的函数， data是要输入的数据，可以传给func
    */
	void (*func)(unsigned long);
	unsigned long data;
};

enum
{
    /*
    此微任务以及进度调度准备执行，而该数据结构已经放在HI_SOFTIRQ或者TASKLET_SOFTIRQ(依赖于所分派的优先级)关联的列表中. 相同微任务不能同时在不同的CPU上调度。当第一个微任务还没开始执行之前，其他的执行微任务的请求又进来了，这些请求会被丢弃。因为对微任务而言，都只能只有一个实例在执行中，所以，没有理由在调度而多执行一次
    */
	TASKLET_STATE_SCHED,	/* Tasklet is scheduled for execution */
    /*
    此微任务正在被执行中，此标识用于防止相同的微任务的多个实例被同时执行，这一点只对SMP系统有意义。此标识的操作通过三个锁完成。 tasklet_trylock, tasklet_unlock, tasklet_unlock_wait.
    */
	TASKLET_STATE_RUN	/* Tasklet is running (SMP only) */
};

```
&emsp;来自处理微任务的重要内核函数
- `tasklet_init`
> 把自身func和data值传入到`tasklet_struct`里面
- `tasklet_action, tasklet_hi_action`
> 分别执行低优先级和高优先级的微任务
- `tasklet_schedule, tasklet_hi_schedule`
> 为低优先级和高优先级的微任务调度以准备执行。这些函数会把`tasklet_struct`结构添加到与本地CPU相关的未决微任务列表里面, 然后为相关的软irq(TASKLET_SOFTIRQ或者HI_SOFTIRQ  )调度。如果微任务已经进入调度(但是还没开始运行)这些API则直接返回(什么都不做)
- `tasklet_eanble, tasklet_hi_enable`
> 这两个函数都一样，用来开启微任务
- `tasklet_diable, tasklet_disable_nosync`
>  这两个函数都是用来关闭微任务的，而且可以用于高优先级和低优先级的微任务. `tasklet_diable`是包含了`tasklet_disable_nosync`函数的，当`tasklet_disable_nosync`(异步返回)只有当微任务已终止其执行时，tasklet_disable 才会返回



## 软irq init
&emsp;kernelinit期间，`softirq_init` 会通过两个通用的软IRQ对软irq层初始化:`tasklet_action`和`tasklet_hi_action`(分别与TASKLET_SOFTIRQ   以及 HI_SOFTIRQ) 关联
```c
void __init softirq_init(void)
{
	int cpu;

	for_each_possible_cpu(cpu) {
		int i;

		per_cpu(tasklet_vec, cpu).tail =
			&per_cpu(tasklet_vec, cpu).head;
		per_cpu(tasklet_hi_vec, cpu).tail =
			&per_cpu(tasklet_hi_vec, cpu).head;
		for (i = 0; i < NR_SOFTIRQS; i++)
			INIT_LIST_HEAD(&per_cpu(softirq_work_list[i], cpu));
	}

	register_hotcpu_notifier(&remote_softirq_cpu_notifier);

	open_softirq(TASKLET_SOFTIRQ, tasklet_action);
	open_softirq(HI_SOFTIRQ, tasklet_hi_action);
}
```
&emsp; 两个网络代码`NET_RX_SOFTIRQ`和`NET_TX_SOFTIRQ` 使用的软irq是在`net_dev_init`里面初始化的.
&emsp;`HI_SOFTIRQ`一般是由声卡设备驱动程序使用的APACHE
&emsp;`TASKLET_SOFTIRQ`使用者一般包括如下:
- 网络适配卡的驱动程序(不局限于Ethernet)
- 各种其他设备驱动程序
- 媒介层(USB, IEEE 1394...)
- 网络子系统(邻居子系统,ATM qdisc等)

## 未决软IRQ的处理
&emsp;`do_softirq` 何时会被启用以负责那些未决的软irq. 如果CPU正在服务于硬件或者软中断,`do_softirq`就会停止而什么都不做，此函数会调用in_interrupt(相当于`in_irq`和`in_softirq`)来检查这些事情.
&emsp;如果`do_softirq`决定继续下去，就会以`local_softirq_pending`把未决的软irq存储在pending里面。

```c 
asmlinkage void do_softirq(void)
{
	__u32 pending;
	unsigned long flags;

	if (in_interrupt())
		return;

	local_irq_save(flags);
	pending = local_softirq_pending();
	/* Switch to interrupt stack */
	if (pending) {
		call_softirq();  //  这一段是汇编指令，本质上是调用`__do_softirq`
		WARN_ON_ONCE(softirq_count());
	}
	local_irq_restore(flags);
}
```

&emsp;上述片段来看,`do_softirq`执行似乎`IRQ`是关闭的,但是并非如此,只有在操作未决的软IRQ位图是，irq才会被关闭,(也就是在访问softnet_data结构时),执行软IRQ函数时候,`__do_softirq`会在内部重新打开irq.


#### `__do_softirq`函数
&emsp;在执行`do_softirq`运行时候，同一种软irq类型会被调度多次。由于在执行软irq函数时，irq会被开启，而服务于一个中断事件时, 未决的软irq的位图也可以受到操作，因此任何已经被`__do_softirq`执行的软`irq`函数在`__do_softirq`本身执行期间可能会被重新调度
&emsp;因此`__do_softirq`在重启IRQ之前，会被未决的软IRQ的当前位图存储到局部变量pending里面，r然后调用`local_softirq_pending() = 0` 将其从与本地cpu相关联的`softnet_data`实力里面清除掉， 最后在根据pending调用所有必需的处理函数

&emsp; 一旦所有处理的函数已经被调用了,`__do_softirq`会同时检查是否有任何软IRQ已经进入调度。如果至少有一个未决的软IRQ，则会重复整个流程. 然而`__do_softirq` 只会重复最多MAX_SOFTIRQ_RESTART次

&emsp;使用`MAX_SOFTIRQ_RESTART` 是一种设计决策，为的在几个CPU中的其中之一上，网络流中的某中类型中断事件不会让他们的中断被饿死. 

&emsp;饿死发生的情况: `do_IRQ`引发了一个`NET_RX_SOFTIRQ`中断事件,使得`do_softirq`被执行.`__do_softirq`会清除`NET_RX_SOFTIRQ`标识,但是结束之前可能会被另外一个中断事件中断，使得NET_RX_SOFTIRQ   会被再次设置，结果就是死循环

&emsp;现在看`__do_softirq`的关键部分是如何启动软IRQ的，每一次一种软irq类型受到服务的时候,其位就会从活跃的软 irq本地副本pending里面被清除掉.h 会初始化为指向全局的数据结构的`softirq_vec`，该数据结构持有软IRQ类型及其handler(例如NET_RX_SOFTIRQ 由net_rx_action处理),当位图被清除的此循环就结束了

&emsp;最后如果因为`do_softirq`已经重复其工作MAX_SOFTIRQ_RESTART次，有些未决的软irq仍然无法被处理，必须返回，则ksoftirq线程会被唤醒，赋予稍后处理这些irq的责任。因为`do_softirq`h会在内核许多地方启用，实际很有可能在ksoftirqd线程调度之前，晚一点调用的do_softirq就会处理这些中断事件。



## ksoftirqd 内核线程
&emsp;后台内核线程被分派的工作是检查没被前述函数执行到的软irq.然后在必须把cpu交还给其他活动之前，尽可能的多执行几个没被执行的软irq. 每个CPU都有这么一个内核线程，名为`ksoftirqd_cpu0`,`ksoftirqd_cpu1`等。(后面启动线程会说明线程CPU引导期间是如何工作的)



## 网络代码如何使用软IRQ
&emsp;网络子系统分派两种不同的软IRQ，`NET_RX_SOFTIRQ`会处理进来的流量, 而`NET_TX_SOFTIRQ` 会处理出去的流量.(两个网络软irq的优先级要比普通任务要高,但是比高优先级的微任务要低). 这样的优先级安排，即使处理高网路哦负载的情况下，也能保证其他高优先级的任务的运行具有足够的响应力和即使的效果;



## `softnet_data`
&emsp;每个CPU都有其队列，用来接收进来的帧，因为每个CPU都有其数据结构来处理进来和出去的流量(不需要上锁，因为每个CPU都有一个)
```c 
struct softnet_data {
	struct Qdisc		*output_queue;
	struct Qdisc		**output_queue_tailp;
	struct list_head	poll_list;
	struct sk_buff		*completion_queue;
	struct sk_buff_head	process_queue;

	/* stats */
	unsigned int		processed;
	unsigned int		time_squeeze;
	unsigned int		cpu_collision;
	unsigned int		received_rps;

#ifdef CONFIG_RPS
	struct softnet_data	*rps_ipi_list;

	/* Elements below can be accessed between CPUs for RPS */
	struct call_single_data	csd ____cacheline_aligned_in_smp;
	struct softnet_data	*rps_ipi_next;
	unsigned int		cpu;
	unsigned int		input_queue_head;
	unsigned int		input_queue_tail;
#endif
	unsigned int		dropped;
	struct sk_buff_head	input_pkt_queue;
	struct napi_struct	backlog;
};
```
> 此结构可以用于接收和传输。换而言之，NET_RX_SOFTIRQ 和 NET_TX_SOFTIRQ软irq都引用此结构

### softnet_data 字段
&emsp;字段说明:
- `throttle`
> return cpu超负荷? true, false; 值以来于`input_pkt_queue`中帧的数目.当throttle标识设置时, 此CPU所接收的输入帧全部被丢弃(无论队列中帧数据有多少)
- `avg_blog`
> 代表`input_pkt_queue` 队列长度加权后均值，其值介于[0, `netdev_max_backlog`]之间. `avg_blog` 可以用于计算`cng_level`
- `cng_level`
> 代表拥塞等级 , `cng_level` 和`avg_blog` 是与CPU相关的，可以用于非NAPI设备
- `input_pkt_queue`
> 
- `backlog_dev`
- `poll_list`
- `output_queue`
- `completion_queue`


#### `softnet_data`初始化
&emsp;`softnet_data`结构是在`net_dev_init`在引导期间执行初始化的.
```c 
for_each_possible_cpu(i) {
    struct softnet_data *sd = &per_cpu(softnet_data, i);

    memset(sd, 0, sizeof(*sd));
    skb_queue_head_init(&sd->input_pkt_queue);
    skb_queue_head_init(&sd->process_queue);
    sd->completion_queue = NULL;
    INIT_LIST_HEAD(&sd->poll_list);
    sd->output_queue = NULL;
    sd->output_queue_tailp = &sd->output_queue;
#ifdef CONFIG_RPS
    sd->csd.func = rps_trigger_softirq;
    sd->csd.info = sd;
    sd->csd.flags = 0;
    sd->cpu = i;
#endif

    sd->backlog.poll = process_backlog;
    sd->backlog.weight = weight_p;
    sd->backlog.gro_list = NULL;
    sd->backlog.gro_count = 0;
}
```
