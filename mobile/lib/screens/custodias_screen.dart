import 'package:flutter/material.dart';

import '../models/custodia.dart';
import '../services/portfolio_repository.dart';

/// CRUD simples — sem edição, só criar/listar/excluir, mesmo nível de
/// simplicidade de `desktop/src-tauri/src/commands/custodia.rs`.
class CustodiasScreen extends StatefulWidget {
  const CustodiasScreen({super.key});

  @override
  State<CustodiasScreen> createState() => _CustodiasScreenState();
}

class _CustodiasScreenState extends State<CustodiasScreen> {
  final _repository = PortfolioRepository();
  final _instituicaoController = TextEditingController();
  final _titularController = TextEditingController();

  bool _loading = true;
  bool _saving = false;
  List<Custodia> _custodias = [];

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    final custodias = await _repository.listCustodias();
    setState(() {
      _custodias = custodias;
      _loading = false;
    });
  }

  Future<void> _add() async {
    final instituicao = _instituicaoController.text.trim();
    final titular = _titularController.text.trim();
    if (instituicao.isEmpty || titular.isEmpty) return;

    setState(() => _saving = true);

    await _repository.insertCustodia(
      Custodia(
        instituicao: instituicao,
        titular: titular,
        createdAt: DateTime.now(),
      ),
    );

    _instituicaoController.clear();
    _titularController.clear();
    setState(() => _saving = false);
    await _load();
  }

  Future<void> _delete(int id) async {
    await _repository.deleteCustodia(id);
    await _load();
  }

  @override
  void dispose() {
    _instituicaoController.dispose();
    _titularController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Custódias')),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _instituicaoController,
                    decoration: const InputDecoration(
                      labelText: 'Instituição',
                      hintText: 'XP',
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: TextField(
                    controller: _titularController,
                    decoration: const InputDecoration(labelText: 'Titular'),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            FilledButton(
              onPressed: _saving ? null : _add,
              child: const Text('Adicionar'),
            ),
            const SizedBox(height: 24),
            Expanded(
              child: _loading
                  ? const Center(child: CircularProgressIndicator())
                  : _custodias.isEmpty
                  ? const Center(
                      child: Text('Nenhuma custódia cadastrada ainda.'),
                    )
                  : ListView.builder(
                      itemCount: _custodias.length,
                      itemBuilder: (context, index) {
                        final custodia = _custodias[index];
                        return Card(
                          margin: const EdgeInsets.only(bottom: 8),
                          child: ListTile(
                            title: Text(custodia.instituicao),
                            subtitle: Text(custodia.titular),
                            trailing: IconButton(
                              icon: const Icon(Icons.delete_outline),
                              onPressed: () => _delete(custodia.id!),
                            ),
                          ),
                        );
                      },
                    ),
            ),
          ],
        ),
      ),
    );
  }
}
