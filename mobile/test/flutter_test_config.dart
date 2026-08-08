import 'dart:async';

import 'package:sqflite_common_ffi/sqflite_ffi.dart';

/// `flutter test` roda na VM Dart, sem os plugins de plataforma que o
/// `sqflite` normal precisa — sem isso, qualquer teste que monte uma tela
/// dependente do banco (`PortfolioScreen`) falha com "databaseFactory not
/// initialized". `sqflite_common_ffi` troca a implementação por SQLite de
/// verdade rodando via FFI, mesmo comportamento real, só sem depender de
/// device/emulador.
Future<void> testExecutable(FutureOr<void> Function() testMain) async {
  sqfliteFfiInit();
  databaseFactory = databaseFactoryFfi;
  return testMain();
}
