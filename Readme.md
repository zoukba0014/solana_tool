# Solana_tool

solana链上的脚本集合

## 简介

在项目中遇到，所编写的脚本集合，用于便利各项操作

## 功能

- 批量查询钱包，sol 数量，代币数量
- 批量创建钱包
- 批量查询钱包地址的代币数量
- 批量进行代币转账

## TODO

- 我会写一个cli的框架到时候有新的脚本出来，直接往里面增加命令，填代码就好了, 每增加功能后记得修改下`readme.md`

## 命令

- 帮助命令

```bash
./solana_tool --help
```

- 钱包

```bash
./solana_tool wallet crate --output file_path

./solana_tool wallet balance --sub-keypair-folder folder_path //默认是sol

./solana_tool wallet balance --sub-keypair-folder folder_path --token-address token_mint_address
```

- 私钥转换

```bash
./solana_tool convert bs58 //json file to bs58
./solana_tool convert json //bs58 to json file
```

- 批量转水

```bash

./solana_tool distibute ... //如果是转spl token 记得加 --token_address 参数指向 token_mint 地址
```

- 批量收集

```bash
./solana_tool collect ... //同上
```
