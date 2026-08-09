/// Conta/corretora (ex: "XP (Fábio)") — espelha
/// `desktop/src-tauri/src/entity/custodia.rs`, sem `workspace_id` (o mobile
/// não tem Workspace).
class Custodia {
  final int? id;
  final String instituicao;
  final String titular;
  final DateTime createdAt;

  const Custodia({
    this.id,
    required this.instituicao,
    required this.titular,
    required this.createdAt,
  });

  String get label => '$instituicao ($titular)';

  Map<String, Object?> toMap() {
    return {
      'id': id,
      'instituicao': instituicao,
      'titular': titular,
      'created_at': createdAt.toIso8601String(),
    };
  }

  factory Custodia.fromMap(Map<String, Object?> map) {
    return Custodia(
      id: map['id'] as int,
      instituicao: map['instituicao'] as String,
      titular: map['titular'] as String,
      createdAt: DateTime.parse(map['created_at'] as String),
    );
  }
}
