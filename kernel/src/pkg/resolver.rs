use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::{BTreeMap, BTreeSet};
use super::{PackageError, Result};
use super::format::{PackageInfo, Version, VersionConstraint, Dependency};
use super::database::PackageDatabase;
use super::repository::Repository;

#[derive(Debug, Clone)]
pub struct DependencyResolver {
    available: BTreeMap<String, Vec<PackageInfo>>,
    installed: BTreeMap<String, PackageInfo>,
    provides: BTreeMap<String, Vec<String>>,
    conflicts: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct Resolution {
    pub to_install: Vec<PackageInfo>,
    pub to_upgrade: Vec<(PackageInfo, PackageInfo)>,
    pub to_remove: Vec<PackageInfo>,
    pub conflicts: Vec<Conflict>,
}

#[derive(Debug, Clone)]
pub struct Conflict {
    pub package1: String,
    pub package2: String,
    pub reason: ConflictReason,
}

#[derive(Debug, Clone)]
pub enum ConflictReason {
    ExplicitConflict,
    FileConflict(String),
    VersionConflict(String),
    MissingDependency(String),
    CircularDependency,
}

impl DependencyResolver {
    pub fn new() -> Self {
        Self {
            available: BTreeMap::new(),
            installed: BTreeMap::new(),
            provides: BTreeMap::new(),
            conflicts: BTreeMap::new(),
        }
    }

    pub fn load_installed(&mut self, db: &PackageDatabase) {
        for pkg in db.list_installed() {
            self.installed.insert(pkg.name.clone(), pkg.clone());
            
            for provided in &pkg.provides {
                self.provides.entry(provided.clone())
                    .or_insert_with(Vec::new)
                    .push(pkg.name.clone());
            }
            
            for conflict in &pkg.conflicts {
                self.conflicts.entry(pkg.name.clone())
                    .or_insert_with(Vec::new)
                    .push(conflict.clone());
            }
        }
    }

    pub fn load_available(&mut self, repos: &[Repository]) {
        self.available.clear();
        
        for repo in repos {
            for pkg in repo.list_packages() {
                self.available.entry(pkg.name.clone())
                    .or_insert_with(Vec::new)
                    .push(pkg);
            }
        }

        for versions in self.available.values_mut() {
            versions.sort_by(|a, b| b.version.cmp(&a.version));
        }
    }

    pub fn resolve_install(&self, package_name: &str) -> Result<Resolution> {
        let mut resolution = Resolution {
            to_install: Vec::new(),
            to_upgrade: Vec::new(),
            to_remove: Vec::new(),
            conflicts: Vec::new(),
        };

        let mut to_process = vec![package_name.to_string()];
        let mut processed = BTreeSet::new();
        let mut selected = BTreeMap::new();

        while let Some(name) = to_process.pop() {
            if processed.contains(&name) {
                continue;
            }
            processed.insert(name.clone());

            if self.installed.contains_key(&name) {
                continue;
            }

            let pkg = self.find_best_version(&name, None)?;

            if let Some(conflicts) = self.check_conflicts(&pkg, &selected) {
                resolution.conflicts.extend(conflicts);
                return Ok(resolution);
            }

            for dep in &pkg.dependencies {
                if !dep.optional {
                    let resolved = self.resolve_dependency(&dep, &selected)?;
                    if !processed.contains(&resolved) {
                        to_process.push(resolved);
                    }
                }
            }

            selected.insert(name.clone(), pkg.clone());
        }

        for (_, pkg) in selected {
            resolution.to_install.push(pkg);
        }

        self.check_circular_dependencies(&resolution)?;

        Ok(resolution)
    }

    pub fn resolve_upgrade(&self, package_name: Option<&str>) -> Result<Resolution> {
        let mut resolution = Resolution {
            to_install: Vec::new(),
            to_upgrade: Vec::new(),
            to_remove: Vec::new(),
            conflicts: Vec::new(),
        };

        let packages_to_upgrade = if let Some(name) = package_name {
            vec![name.to_string()]
        } else {
            self.installed.keys().cloned().collect()
        };

        for name in packages_to_upgrade {
            if let Some(old) = self.installed.get(&name) {
                if let Ok(new) = self.find_best_version(&name, Some(&old.version)) {
                    if new.version > old.version {
                        resolution.to_upgrade.push((old.clone(), new));
                    }
                }
            }
        }

        Ok(resolution)
    }

    pub fn resolve_remove(&self, package_name: &str) -> Result<Resolution> {
        let mut resolution = Resolution {
            to_install: Vec::new(),
            to_upgrade: Vec::new(),
            to_remove: Vec::new(),
            conflicts: Vec::new(),
        };

        let pkg = self.installed.get(package_name)
            .ok_or_else(|| PackageError::NotFound(package_name.to_string()))?;

        resolution.to_remove.push(pkg.clone());

        let dependents = self.find_dependents(package_name);
        for dep in dependents {
            if let Some(dep_pkg) = self.installed.get(&dep) {
                resolution.to_remove.push(dep_pkg.clone());
            }
        }

        Ok(resolution)
    }

    fn find_best_version(&self, name: &str, min_version: Option<&Version>) -> Result<PackageInfo> {
        if let Some(providers) = self.provides.get(name) {
            for provider in providers {
                if let Some(versions) = self.available.get(provider) {
                    if let Some(pkg) = versions.first() {
                        return Ok(pkg.clone());
                    }
                }
            }
        }

        let versions = self.available.get(name)
            .ok_or_else(|| PackageError::NotFound(name.to_string()))?;

        let pkg = if let Some(min) = min_version {
            versions.iter()
                .find(|p| p.version > *min)
                .or_else(|| versions.first())
        } else {
            versions.first()
        };

        pkg.cloned()
            .ok_or_else(|| PackageError::NotFound(name.to_string()))
    }

    fn resolve_dependency(&self, dep: &Dependency, selected: &BTreeMap<String, PackageInfo>) -> Result<String> {
        if selected.contains_key(&dep.name) {
            return Ok(dep.name.clone());
        }

        if let Some(pkg) = self.installed.get(&dep.name) {
            if dep.constraint.matches(&pkg.version) {
                return Ok(dep.name.clone());
            }
        }

        if let Some(providers) = self.provides.get(&dep.name) {
            for provider in providers {
                if let Some(pkg) = self.installed.get(provider) {
                    if dep.constraint.matches(&pkg.version) {
                        return Ok(provider.clone());
                    }
                }
                
                if let Some(versions) = self.available.get(provider) {
                    for pkg in versions {
                        if dep.constraint.matches(&pkg.version) {
                            return Ok(provider.clone());
                        }
                    }
                }
            }
        }

        if let Some(versions) = self.available.get(&dep.name) {
            for pkg in versions {
                if dep.constraint.matches(&pkg.version) {
                    return Ok(dep.name.clone());
                }
            }
        }

        Err(PackageError::DependencyConflict(
            format!("Cannot resolve dependency: {} {}", dep.name, dep.constraint)
        ))
    }

    fn check_conflicts(&self, pkg: &PackageInfo, selected: &BTreeMap<String, PackageInfo>) -> Option<Vec<Conflict>> {
        let mut conflicts = Vec::new();

        for conflict_name in &pkg.conflicts {
            if self.installed.contains_key(conflict_name) {
                conflicts.push(Conflict {
                    package1: pkg.name.clone(),
                    package2: conflict_name.clone(),
                    reason: ConflictReason::ExplicitConflict,
                });
            }

            if selected.contains_key(conflict_name) {
                conflicts.push(Conflict {
                    package1: pkg.name.clone(),
                    package2: conflict_name.clone(),
                    reason: ConflictReason::ExplicitConflict,
                });
            }
        }

        if let Some(pkg_conflicts) = self.conflicts.get(&pkg.name) {
            for conflict in pkg_conflicts {
                if self.installed.contains_key(conflict) || selected.contains_key(conflict) {
                    conflicts.push(Conflict {
                        package1: pkg.name.clone(),
                        package2: conflict.clone(),
                        reason: ConflictReason::ExplicitConflict,
                    });
                }
            }
        }

        if !conflicts.is_empty() {
            Some(conflicts)
        } else {
            None
        }
    }

    fn check_circular_dependencies(&self, resolution: &Resolution) -> Result<()> {
        let mut graph = BTreeMap::new();
        
        for pkg in &resolution.to_install {
            let deps: Vec<String> = pkg.dependencies.iter()
                .filter(|d| !d.optional)
                .map(|d| d.name.clone())
                .collect();
            graph.insert(pkg.name.clone(), deps);
        }

        for (name, _) in &graph {
            if self.has_circular_dependency(name, name, &graph, &mut BTreeSet::new()) {
                return Err(PackageError::DependencyConflict(
                    format!("Circular dependency detected involving {}", name)
                ));
            }
        }

        Ok(())
    }

    fn has_circular_dependency(
        &self,
        start: &str,
        current: &str,
        graph: &BTreeMap<String, Vec<String>>,
        visited: &mut BTreeSet<String>
    ) -> bool {
        if visited.contains(current) {
            return current == start;
        }

        visited.insert(current.to_string());

        if let Some(deps) = graph.get(current) {
            for dep in deps {
                if self.has_circular_dependency(start, dep, graph, visited) {
                    return true;
                }
            }
        }

        visited.remove(current);
        false
    }

    fn find_dependents(&self, package: &str) -> Vec<String> {
        let mut dependents = Vec::new();

        for (name, pkg) in &self.installed {
            for dep in &pkg.dependencies {
                if dep.name == package && !dep.optional {
                    dependents.push(name.clone());
                    break;
                }
            }
        }

        dependents
    }

    pub fn calculate_download_size(&self, resolution: &Resolution) -> u64 {
        let mut size = 0u64;

        for pkg in &resolution.to_install {
            size += pkg.size;
        }

        for (_, new) in &resolution.to_upgrade {
            size += new.size;
        }

        size
    }

    pub fn calculate_install_size(&self, resolution: &Resolution) -> i64 {
        let mut size = 0i64;

        for pkg in &resolution.to_install {
            size += pkg.installed_size as i64;
        }

        for (old, new) in &resolution.to_upgrade {
            size += new.installed_size as i64 - old.installed_size as i64;
        }

        for pkg in &resolution.to_remove {
            size -= pkg.installed_size as i64;
        }

        size
    }

    pub fn get_install_order(&self, packages: &[PackageInfo]) -> Result<Vec<PackageInfo>> {
        let mut ordered = Vec::new();
        let mut visited = BTreeSet::new();
        let mut temp_mark = BTreeSet::new();

        let mut graph = BTreeMap::new();
        for pkg in packages {
            let deps: Vec<String> = pkg.dependencies.iter()
                .filter(|d| !d.optional)
                .map(|d| d.name.clone())
                .filter(|name| packages.iter().any(|p| &p.name == name))
                .collect();
            graph.insert(pkg.name.clone(), (pkg.clone(), deps));
        }

        for pkg in packages {
            if !visited.contains(&pkg.name) {
                self.topological_sort(&pkg.name, &graph, &mut visited, &mut temp_mark, &mut ordered)?;
            }
        }

        ordered.reverse();
        Ok(ordered)
    }

    fn topological_sort(
        &self,
        name: &str,
        graph: &BTreeMap<String, (PackageInfo, Vec<String>)>,
        visited: &mut BTreeSet<String>,
        temp_mark: &mut BTreeSet<String>,
        ordered: &mut Vec<PackageInfo>
    ) -> Result<()> {
        if temp_mark.contains(name) {
            return Err(PackageError::DependencyConflict(
                format!("Circular dependency detected at {}", name)
            ));
        }

        if visited.contains(name) {
            return Ok(());
        }

        temp_mark.insert(name.to_string());

        if let Some((pkg, deps)) = graph.get(name) {
            for dep in deps {
                self.topological_sort(dep, graph, visited, temp_mark, ordered)?;
            }
            
            visited.insert(name.to_string());
            ordered.push(pkg.clone());
        }

        temp_mark.remove(name);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SATSolver {
    variables: BTreeMap<String, usize>,
    clauses: Vec<Vec<i32>>,
    assignment: Vec<Option<bool>>,
}

impl SATSolver {
    pub fn new() -> Self {
        Self {
            variables: BTreeMap::new(),
            clauses: Vec::new(),
            assignment: Vec::new(),
        }
    }

    pub fn add_variable(&mut self, name: String) -> usize {
        let id = self.variables.len();
        self.variables.insert(name, id);
        self.assignment.push(None);
        id
    }

    pub fn add_clause(&mut self, clause: Vec<i32>) {
        self.clauses.push(clause);
    }

    pub fn solve(&mut self) -> bool {
        self.dpll(0)
    }

    fn dpll(&mut self, level: usize) -> bool {
        if self.all_clauses_satisfied() {
            return true;
        }

        if self.has_empty_clause() {
            return false;
        }

        if let Some(unit) = self.find_unit_clause() {
            self.assign(unit.abs() as usize - 1, unit > 0);
            let result = self.dpll(level + 1);
            if !result {
                self.unassign(unit.abs() as usize - 1);
            }
            return result;
        }

        if let Some(var) = self.choose_variable() {
            self.assign(var, true);
            if self.dpll(level + 1) {
                return true;
            }
            self.unassign(var);

            self.assign(var, false);
            if self.dpll(level + 1) {
                return true;
            }
            self.unassign(var);
        }

        false
    }

    fn all_clauses_satisfied(&self) -> bool {
        self.clauses.iter().all(|clause| self.clause_satisfied(clause))
    }

    fn clause_satisfied(&self, clause: &[i32]) -> bool {
        clause.iter().any(|&lit| {
            let var = (lit.abs() - 1) as usize;
            self.assignment[var] == Some(lit > 0)
        })
    }

    fn has_empty_clause(&self) -> bool {
        self.clauses.iter().any(|clause| {
            clause.iter().all(|&lit| {
                let var = (lit.abs() - 1) as usize;
                self.assignment[var] == Some(lit < 0)
            })
        })
    }

    fn find_unit_clause(&self) -> Option<i32> {
        for clause in &self.clauses {
            let unassigned: Vec<_> = clause.iter()
                .filter(|&&lit| {
                    let var = (lit.abs() - 1) as usize;
                    self.assignment[var].is_none()
                })
                .collect();

            if unassigned.len() == 1 {
                let satisfied = clause.iter().any(|&lit| {
                    let var = (lit.abs() - 1) as usize;
                    self.assignment[var] == Some(lit > 0)
                });

                if !satisfied {
                    return Some(**unassigned[0]);
                }
            }
        }
        None
    }

    fn choose_variable(&self) -> Option<usize> {
        self.assignment.iter()
            .position(|a| a.is_none())
    }

    fn assign(&mut self, var: usize, value: bool) {
        self.assignment[var] = Some(value);
    }

    fn unassign(&mut self, var: usize) {
        self.assignment[var] = None;
    }

    pub fn get_solution(&self) -> BTreeMap<String, bool> {
        let mut solution = BTreeMap::new();
        for (name, &id) in &self.variables {
            if let Some(value) = self.assignment[id] {
                solution.insert(name.clone(), value);
            }
        }
        solution
    }
}