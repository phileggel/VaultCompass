##

Use Case (Application) : C'est le chef d'orchestre. Il définit les étapes d'une action utilisateur (ex: "S'inscrire"). Il ne connaît pas SQL, mais il décide où commence et où finit la transaction. Il est spécifique à une action de ton application.

Domain Service (Domaine) : C'est l'expert métier. Il contient des règles qui ne peuvent pas vivre dans une seule entité (ex: "Vérifier la cohérence entre un stock et une commande"). Il est agnostique et réutilisable par plusieurs Use Cases.

Repository Trait (Domaine) : C'est le contrat. Il définit quelles données on peut lire ou écrire, sans dire comment.

Entité (Domaine) : C'est le cœur. Elle contient les données et les règles métier internes (ex: "Un prix ne peut pas être négatif").

Infrastructure (SQLx) : C'est la main-d'œuvre. Elle implémente les traits du domaine en parlant concrètement à la base de données.

## Transaction example

src/
├── domain/
│ ├── models.rs (User, Badge)
│ └── repositories.rs (Traits UserRepository, BadgeRepository)
├── application/
│ ├── ports.rs (Trait TransactionManager, Trait AppUnitOfWork)
│ └── use_cases/
│ └── register.rs (Struct RegisterUserUseCase { tx_manager: Arc<dyn TransactionManager> })
├── infrastructure/
│ └── db/
│ ├── postgres.rs (Struct PostgresTxManager, Struct SqlxUnitOfWork)
│ └── repos_impl/ (Implémentations concrètes)
└── main.rs (Initialisation et injection)

use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;

// ==========================================
// 1. COUCHE DOMAINE (Contrats et Logique pure)
// ==========================================

pub struct User { pub id: i32, pub email: String }
pub struct Badge { pub label: String }

#[async_trait]
pub trait UserRepository: Send {
async fn save(&mut self, email: String) -> Result<User, String>;
}

#[async_trait]
pub trait BadgeRepository: Send {
async fn assign_welcome_badge(&mut self, user_id: i32) -> Result<(), String>;
}

// Le super-trait qui regroupe tout pour la transaction
pub trait AppUnitOfWork: UserRepository + BadgeRepository + Send {}

// Service de Domaine : Logique métier complexe
pub struct RegistrationDomainService;
impl RegistrationDomainService {
pub fn validate_email(&self, email: &str) -> Result<(), String> {
if !email.contains('@') { return Err("Email invalide".into()); }
Ok(())
}
}

// ==========================================
// 2. COUCHE APPLICATION (Use Case & Agnosticisme)
// ==========================================

pub type UoWFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'a>>;

#[async_trait]
pub trait TransactionManager: Send + Sync {
async fn run<T, F>(&self, operation: F) -> Result<T, String>
where F: for<'a> FnOnce(&'a mut dyn AppUnitOfWork) -> UoWFuture<'a, T> + Send;
}

pub struct RegisterUserUseCase<'a> {
tx_manager: &'a dyn TransactionManager,
domain_service: RegistrationDomainService, // Injecté ou instancié
}

impl<'a> RegisterUserUseCase<'a> {
pub fn new(tx_manager: &'a dyn TransactionManager) -> Self {
Self { tx_manager, domain_service: RegistrationDomainService }
}

    pub async fn execute(&self, email: String) -> Result<User, String> {
        // Règle métier simple avant de lancer la machine lourde
        self.domain_service.validate_email(&email)?;

        // Début de l'orchestration transactionnelle
        self.tx_manager.run(move |uow| Box::pin(async move {
            // 1. Création de l'utilisateur via Repo
            let user = uow.save(email).await?;

            // 2. Logique additionnelle via un autre Repo
            uow.assign_welcome_badge(user.id).await?;

            Ok(user)
        })).await
    }

}

// ==========================================
// 3. COUCHE INFRASTRUCTURE (Détails SQLx)
// ==========================================

pub struct SqlxUnitOfWork<'a> {
tx: sqlx::Transaction<'a, sqlx::Postgres>,
}

#[async_trait]
impl<'a> UserRepository for SqlxUnitOfWork<'a> {
async fn save(&mut self, email: String) -> Result<User, String> {
let row = sqlx::query!("INSERT INTO users (email) VALUES ($1) RETURNING id", email)
.fetch_one(&mut \*self.tx).await.map_err(|e| e.to_string())?;
Ok(User { id: row.id, email })
}
}

#[async_trait]
impl<'a> BadgeRepository for SqlxUnitOfWork<'a> {
async fn assign_welcome_badge(&mut self, user_id: i32) -> Result<(), String> {
sqlx::query!("INSERT INTO badges (user_id, label) VALUES ($1, $2)", user_id, "Welcome")
.execute(&mut \*self.tx).await.map_err(|e| e.to_string())?;
Ok(())
}
}

impl<'a> AppUnitOfWork for SqlxUnitOfWork<'a> {}

pub struct PostgresTxManager { pub pool: sqlx::PgPool }

#[async_trait]
impl TransactionManager for PostgresTxManager {
async fn run<T, F>(&self, operation: F) -> Result<T, String>
where F: for<'a> FnOnce(&'a mut dyn AppUnitOfWork) -> UoWFuture<'a, T> + Send
{
let tx = self.pool.begin().await.map_err(|e| e.to_string())?;
let mut uow = SqlxUnitOfWork { tx };

        let result = operation(&mut uow).await;

        if result.is_ok() {
            uow.tx.commit().await.map_err(|e| e.to_string())?;
        } else {
            uow.tx.rollback().await.map_err(|e| e.to_string())?;
        }
        result
    }

}
