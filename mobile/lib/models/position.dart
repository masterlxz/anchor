import 'asset.dart';

/// Calculado a partir do histórico de transações, nunca persistido —
/// mesmo espírito de `get_portfolio_positions` no desktop
/// (`desktop/src-tauri/src/commands/transaction.rs`).
class Position {
  final Asset asset;
  final double netQuantity;

  /// `null` quando o ativo nunca teve uma compra registrada.
  final double? averageBuyPrice;

  const Position({
    required this.asset,
    required this.netQuantity,
    required this.averageBuyPrice,
  });
}
