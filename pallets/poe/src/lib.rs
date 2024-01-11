#![cfg_attr(not(feature = "std"), no_std)] /// 必须声明在第一行，前面不能有任何内容 如果依赖库有std标准库，就用标准库，没有就不用即no_std

/// 导出poe模块定义的内容，比如存储单元，给外部使用
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

use frame_support::pallet_prelude::*; // 包含了get等接口和一些常用的宏
use frame_system::pallet_prelude::*; // 包含了ensure_signed等
use sp_std::prelude::*;  // 包含了集合类型
pub use weights::WeightInfo;

/// 必须引入以下两个宏，才能对poe模块进行单元测试
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	
	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)] 
	pub struct Pallet<T>(_);

	/// 定义配置
	/// 模块配置接口Config（在rust中使用trait来定义接口）
	/// Config接口继承自frame_system::Config，这样Config就拥有了系统定义的数据类型，比如BlockNumber，哈希类型Hashing，AccountId
	/// #[pallet::config]为宏
	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// 事件
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		/// 存证内容的最大长度
		/// 因为MaxClaimLength是一个常量，所以这里我们需要用到#[pallet::constant]常量宏，来声明这是一个常量
		#[pallet::constant]
		type MaxClaimLength: Get<u32>;
	}

	// The pallet's runtime storage items.
	// https://docs.substrate.io/main-docs/build/runtime-storage/
	#[pallet::storage]
	/// 定义存证结构
	#[pallet::getter(fn proofs)]
	pub type Proofs<T: Config> = StorageMap<
		_,
		Blake2_128Concat, // 键 哈希算法
		BoundedVec<u8, T::MaxClaimLength>, // 在新版中，不能直接使用Vec，而是使用更安全的BoundedVec
		(T::AccountId, BlockNumberFor<T>), // 值 定义成了一个元组格式
	>;

	// 定义事件
	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// 创建存证 [who, hash]
		ClaimCreated(T::AccountId, BoundedVec<u8, T::MaxClaimLength>),

		/// 撤销存证 [who, hash]
		ClaimRevoked(T::AccountId, BoundedVec<u8, T::MaxClaimLength>),

		/// 转移存证 [who, to, hash]
		ClaimTransfered(T::AccountId, T::AccountId, BoundedVec<u8, T::MaxClaimLength>),
	}

	// 定义Error
	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// 存证已经存在
		ClaimAlreadyExist,
		/// 存证不存在
		ClaimNotExist,
		/// 不是存证owner
		NotClaimOwner,
	}
	/// 定义钩子
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	/// 定义可调用函数
	/// 可调度函数允许用户与pallet交互并调用状态更改
	/// 这些函数具体化为“外部函数”，通常与交易进行比较
	/// Dispatchable 函数必须设置权重，并且必须返回 DispatchResult
	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// 创建存证
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_claim(claim.len() as u32))]
		/// 加pub后，为公共方法，默认为private
		pub fn create_claim(origin: OriginFor<T>, claim: BoundedVec<u8, T::MaxClaimLength>) -> DispatchResultWithPostInfo {
			
			// 验签
			let sender = ensure_signed(origin)?;
			
			// 检查是否已经存在该存证 
			// 只有条件为true时，才不会报后面的Error，ensure!()相当于solidity中的require()
			ensure!(!Proofs::<T>::contains_key(&claim), Error::<T>::ClaimAlreadyExist);

			// 插入数据
			Proofs::<T>::insert(
				&claim, // 键
				(sender.clone(), frame_system::Pallet::<T>::block_number()), // 值，元组格式
			);

			// 触发事件
			Self::deposit_event(Event::ClaimCreated(sender, claim));

			// 返回OK
			Ok(().into())

		}

		/// 撤销存证
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::revoke_claim(claim.len() as u32))]
		pub fn revoke_claim(origin: OriginFor<T>, claim: BoundedVec<u8, T::MaxClaimLength>) -> DispatchResultWithPostInfo {
			
			// 验签
			let sender = ensure_signed(origin)?;
			
			// 检查是否已经存在该存证
			let (owner, _) = Proofs::<T>::get(&claim).ok_or(Error::<T>::ClaimNotExist)?;

			// 检查当前用户是否为该存证的owner
			ensure!(owner == sender, Error::<T>::NotClaimOwner);

			// 删除存证
			Proofs::<T>::remove(&claim);
			
			// 触发事件
			Self::deposit_event(Event::ClaimRevoked(sender, claim));

			// 返回OK
			Ok(().into())

		}

		/// 转移存证
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::transfer_claim(claim.len() as u32))]
		pub fn transfer_claim(origin: OriginFor<T>, claim: BoundedVec<u8, T::MaxClaimLength>, dest: T::AccountId) -> DispatchResultWithPostInfo {
			
			// 验签
			let sender = ensure_signed(origin)?;
			
			// 检查是否已经存在该存证
			let (owner, _) = Proofs::<T>::get(&claim).ok_or(Error::<T>::ClaimNotExist)?;

			// 检查当前用户是否为该存证的owner
			ensure!(owner == sender, Error::<T>::NotClaimOwner);

			// 转移存证
			Proofs::<T>::insert(&claim, (&dest, frame_system::Pallet::<T>::block_number()));
			
			// 触发事件
			Self::deposit_event(Event::ClaimTransfered(sender, dest, claim));

			// 返回OK
			Ok(().into())

		}
	}
}
