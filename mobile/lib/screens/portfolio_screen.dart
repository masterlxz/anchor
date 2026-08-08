import 'package:flutter/material.dart';

import '../models/position.dart';
import '../services/portfolio_repository.dart';
import '../services/yahoo_quote_service.dart';
import 'add_asset_screen.dart';
import 'add_transaction_screen.dart';

class PortfolioScreen extends StatefulWidget {
  const PortfolioScreen({super.key});

  @override
  State<PortfolioScreen> createState() => _PortfolioScreenState();
}

class _PositionRow {
  final Position position;
  final double? livePrice;

  const _PositionRow(this.position, this.livePrice);
}

class _PortfolioScreenState extends State<PortfolioScreen> {
  final _repository = PortfolioRepository();
  final _quoteService = YahooQuoteService();

  bool _loading = true;
  List<_PositionRow> _rows = [];

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() => _loading = true);

    final positions = await _repository.getPositions();
    final rows = await Future.wait(positions.map((position) async {
      try {
        final quote = await _quoteService.fetchQuote(position.asset.ticker);
        return _PositionRow(position, quote.price);
      } catch (_) {
        return _PositionRow(position, null);
      }
    }));

    if (mounted) {
      setState(() {
        _rows = rows;
        _loading = false;
      });
    }
  }

  Future<void> _openAddAsset() async {
    final saved = await Navigator.of(context).push<bool>(
      MaterialPageRoute(builder: (_) => const AddAssetScreen()),
    );
    if (saved == true) _load();
  }

  Future<void> _openAddTransaction() async {
    final saved = await Navigator.of(context).push<bool>(
      MaterialPageRoute(builder: (_) => const AddTransactionScreen()),
    );
    if (saved == true) _load();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Portfolio'),
        actions: [
          IconButton(
            icon: const Icon(Icons.add_chart),
            tooltip: 'Adicionar ativo',
            onPressed: _openAddAsset,
          ),
        ],
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _rows.isEmpty
              ? _EmptyState(onAddAsset: _openAddAsset)
              : RefreshIndicator(
                  onRefresh: _load,
                  child: ListView.builder(
                    padding: const EdgeInsets.all(16),
                    itemCount: _rows.length,
                    itemBuilder: (context, index) => _PositionCard(row: _rows[index]),
                  ),
                ),
      floatingActionButton: _rows.isEmpty
          ? null
          : FloatingActionButton(
              onPressed: _openAddTransaction,
              child: const Icon(Icons.add),
            ),
    );
  }
}

class _EmptyState extends StatelessWidget {
  final VoidCallback onAddAsset;

  const _EmptyState({required this.onAddAsset});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text('Nenhum ativo cadastrado ainda.'),
            const SizedBox(height: 16),
            FilledButton(onPressed: onAddAsset, child: const Text('Adicionar ativo')),
          ],
        ),
      ),
    );
  }
}

class _PositionCard extends StatelessWidget {
  final _PositionRow row;

  const _PositionCard({required this.row});

  @override
  Widget build(BuildContext context) {
    final position = row.position;
    final quantity = position.netQuantity;
    final avgPrice = position.averageBuyPrice;
    final livePrice = row.livePrice;

    final currentValue = livePrice != null ? quantity * livePrice : null;
    final costBasis = avgPrice != null ? quantity * avgPrice : null;
    final pnl = (currentValue != null && costBasis != null) ? currentValue - costBasis : null;
    final pnlPercent = (pnl != null && costBasis != null && costBasis != 0)
        ? (pnl / costBasis) * 100
        : null;

    final positiveColor = Theme.of(context).colorScheme.primary;
    final negativeColor = Theme.of(context).colorScheme.error;

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(position.asset.ticker,
                        style: Theme.of(context).textTheme.titleMedium),
                    Text(position.asset.name, style: Theme.of(context).textTheme.bodySmall),
                  ],
                ),
                Text('${quantity.toStringAsFixed(quantity == quantity.roundToDouble() ? 0 : 2)} un.'),
              ],
            ),
            const SizedBox(height: 12),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text('Preço médio: ${avgPrice?.toStringAsFixed(2) ?? '—'}'),
                Text('Preço atual: ${livePrice?.toStringAsFixed(2) ?? '—'}'),
              ],
            ),
            const SizedBox(height: 8),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text('Valor: ${currentValue?.toStringAsFixed(2) ?? '—'}'),
                if (pnl != null && pnlPercent != null)
                  Text(
                    '${pnl >= 0 ? '+' : ''}${pnl.toStringAsFixed(2)} '
                    '(${pnlPercent >= 0 ? '+' : ''}${pnlPercent.toStringAsFixed(1)}%)',
                    style: TextStyle(
                      color: pnl >= 0 ? positiveColor : negativeColor,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
