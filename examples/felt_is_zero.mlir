module attributes {llvm.data_layout = ""} {
  llvm.func internal @"dup<felt252>"(%arg0: i256) -> !llvm.struct<(i256, i256)> {
    %0 = llvm.mlir.undef : !llvm.struct<(i256, i256)>
    %1 = llvm.insertvalue %arg0, %0[0] : !llvm.struct<(i256, i256)> 
    %2 = llvm.insertvalue %arg0, %1[1] : !llvm.struct<(i256, i256)> 
    llvm.return %2 : !llvm.struct<(i256, i256)>
  }
  llvm.func internal @"store_temp<felt252>"(%arg0: i256) -> i256 {
    llvm.return %arg0 : i256
  }
  llvm.func internal @felt252_mul(%arg0: i256, %arg1: i256) -> i256 {
    %0 = llvm.mul %arg0, %arg1  : i256
    %1 = llvm.mlir.constant(3618502788666131213697322783095070105623107215331596699973092056135872020481 : i256) : i256
    %2 = llvm.srem %0, %1  : i256
    llvm.br ^bb1(%2 : i256)
  ^bb1(%3: i256):  // pred: ^bb0
    llvm.return %3 : i256
  }
  llvm.func internal @"rename<felt252>"(%arg0: i256) -> i256 {
    llvm.return %arg0 : i256
  }
  llvm.func internal @felt_is_zero_felt_is_zero_mul_if_not_zero(%arg0: i256) -> i256 {
    llvm.br ^bb1(%arg0 : i256)
  ^bb1(%0: i256):  // pred: ^bb0
    %1 = llvm.call @"dup<felt252>"(%0) : (i256) -> !llvm.struct<(i256, i256)>
    %2 = llvm.extractvalue %1[0] : !llvm.struct<(i256, i256)> 
    %3 = llvm.extractvalue %1[1] : !llvm.struct<(i256, i256)> 
    %4 = llvm.mlir.constant(0 : i256) : i256
    %5 = llvm.icmp "eq" %3, %4 : i256
    llvm.cond_br %5, ^bb2, ^bb3(%2 : i256)
  ^bb2:  // pred: ^bb1
    %6 = llvm.mlir.constant(0 : i256) : i256
    %7 = llvm.call @"store_temp<felt252>"(%6) : (i256) -> i256
    llvm.br ^bb4(%7 : i256)
  ^bb3(%8: i256):  // pred: ^bb1
    %9 = llvm.mlir.constant(2 : i256) : i256
    %10 = llvm.call @felt252_mul(%8, %9) : (i256, i256) -> i256
    %11 = llvm.call @"store_temp<felt252>"(%10) : (i256) -> i256
    llvm.br ^bb4(%11 : i256)
  ^bb4(%12: i256):  // 2 preds: ^bb2, ^bb3
    %13 = llvm.call @"rename<felt252>"(%12) : (i256) -> i256
    llvm.return %13 : i256
  }
}